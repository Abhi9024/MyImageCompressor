using MedImgCompress.Config;
using MedImgCompress.Error;

namespace MedImgCompress.Codec;

/// <summary>
/// JPEG 2000 codec implementation.
/// </summary>
public class Jpeg2000Codec : ICodec
{
    /// <summary>
    /// Whether to use reversible (5/3) or irreversible (9/7) wavelet transform.
    /// </summary>
    public bool UseReversible { get; set; } = true;

    public CodecInfo Info => new()
    {
        Name = "JPEG 2000",
        Version = "MVP 0.1",
        SupportsLossless = true,
        SupportsLossy = true,
        SupportsProgressive = true,
        SupportsRoi = false,
        TransferSyntaxLossless = TransferSyntax.Jpeg2000Lossless,
        TransferSyntaxLossy = TransferSyntax.Jpeg2000Lossy
    };

    public CodecCapabilities Capabilities => new()
    {
        MaxBitsPerSample = 16,
        SupportsSigned = true,
        SupportsColor = true,
        SupportsMultiframe = true
    };

    /// <summary>
    /// Create codec configured for lossless compression.
    /// </summary>
    public static Jpeg2000Codec Lossless() => new() { UseReversible = true };

    /// <summary>
    /// Create codec configured for lossy compression.
    /// </summary>
    public static Jpeg2000Codec Lossy() => new() { UseReversible = false };

    public byte[] Encode(ImageData image, CompressionConfig config)
    {
        ValidateImage(image);

        using var stream = new MemoryStream();

        // SOC (Start of Codestream) marker
        stream.Write(new byte[] { 0xFF, 0x4F });

        // SIZ segment
        WriteSizSegment(stream, image);

        // COD segment
        WriteCodSegment(stream, config);

        // QCD segment
        WriteQcdSegment(stream, config);

        // SOT (Start of Tile-Part) marker
        stream.Write(new byte[] { 0xFF, 0x90 });

        // Tile-part header
        int tileLength = 10 + image.PixelData.Length;
        WriteUInt16BE(stream, (ushort)tileLength);
        stream.Write(new byte[] { 0x00, 0x00 }); // Tile index
        WriteUInt32BE(stream, (uint)tileLength);
        stream.Write(new byte[] { 0x00, 0x01 }); // Tile-part index, number of tile-parts

        // SOD (Start of Data) marker
        stream.Write(new byte[] { 0xFF, 0x93 });

        // Compressed tile data
        byte[] compressedData = CompressTileData(image, config);
        stream.Write(compressedData);

        // EOC (End of Codestream) marker
        stream.Write(new byte[] { 0xFF, 0xD9 });

        return stream.ToArray();
    }

    public ImageData Decode(byte[] data, int width, int height, int bitsPerSample, int samplesPerPixel)
    {
        if (data.Length < 4)
            throw new CodecException("Invalid J2K data: too short");

        // Check for SOC marker
        if (data[0] != 0xFF || data[1] != 0x4F)
            throw new CodecException("Invalid J2K data: missing SOC marker");

        // Find SOD marker and extract compressed data
        int pos = 2;
        while (pos < data.Length - 1)
        {
            if (data[pos] == 0xFF && data[pos + 1] == 0x93)
            {
                pos += 2;
                break;
            }
            pos++;
        }

        // Find EOC marker
        int end = data.Length;
        if (data.Length >= 2 && data[^2] == 0xFF && data[^1] == 0xD9)
        {
            end = data.Length - 2;
        }

        if (pos >= end)
            throw new CodecException("Invalid J2K data: no tile data found");

        byte[] compressed = new byte[end - pos];
        Array.Copy(data, pos, compressed, 0, compressed.Length);

        // Decode based on first byte (quantization parameter for lossy)
        byte[] decoded = compressed.Length > 0 && compressed[0] < 16
            ? LossyDecode(compressed, bitsPerSample)
            : LosslessDecode(compressed, bitsPerSample);

        return new ImageData
        {
            Width = width,
            Height = height,
            BitsPerSample = bitsPerSample,
            SamplesPerPixel = samplesPerPixel,
            PixelData = decoded,
            PhotometricInterpretation = string.Empty,
            IsSigned = false
        };
    }

    private void ValidateImage(ImageData image)
    {
        if (image.Width == 0 || image.Height == 0)
            throw new ImageDataException("Invalid image dimensions");

        if (image.PixelData.Length == 0)
            throw new ImageDataException("Empty pixel data");

        int expectedSize = CalculateExpectedSize(image);
        if (image.PixelData.Length < expectedSize)
            throw new ImageDataException(
                $"Pixel data size mismatch: expected at least {expectedSize} bytes, got {image.PixelData.Length}");
    }

    private int CalculateExpectedSize(ImageData image)
    {
        int bytesPerSample = (image.BitsPerSample + 7) / 8;
        return image.Width * image.Height * image.SamplesPerPixel * bytesPerSample;
    }

    private void WriteSizSegment(MemoryStream stream, ImageData image)
    {
        stream.Write(new byte[] { 0xFF, 0x51 }); // SIZ marker

        int components = image.SamplesPerPixel;
        int segLength = 38 + 3 * components;
        WriteUInt16BE(stream, (ushort)segLength);

        // Profile
        stream.Write(new byte[] { 0x00, 0x00 });

        // Image dimensions
        WriteUInt32BE(stream, (uint)image.Width);
        WriteUInt32BE(stream, (uint)image.Height);

        // Image offset
        stream.Write(new byte[] { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 });

        // Tile dimensions
        WriteUInt32BE(stream, (uint)image.Width);
        WriteUInt32BE(stream, (uint)image.Height);

        // Tile offset
        stream.Write(new byte[] { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 });

        // Number of components
        WriteUInt16BE(stream, (ushort)components);

        // Component parameters
        for (int i = 0; i < components; i++)
        {
            byte ssiz = image.IsSigned
                ? (byte)(0x80 | ((image.BitsPerSample - 1) & 0x7F))
                : (byte)((image.BitsPerSample - 1) & 0x7F);
            stream.WriteByte(ssiz);
            stream.WriteByte(0x01); // XRsiz
            stream.WriteByte(0x01); // YRsiz
        }
    }

    private void WriteCodSegment(MemoryStream stream, CompressionConfig config)
    {
        stream.Write(new byte[] { 0xFF, 0x52 }); // COD marker
        stream.Write(new byte[] { 0x00, 0x0C }); // Length

        stream.WriteByte(0x00); // Coding style
        stream.WriteByte(0x00); // Progression order (LRCP)
        WriteUInt16BE(stream, (ushort)config.QualityLayers);
        stream.WriteByte(0x00); // MCT
        stream.WriteByte(0x05); // Decomposition levels
        stream.WriteByte(0x04); // Code-block width exponent
        stream.WriteByte(0x04); // Code-block height exponent
        stream.WriteByte(0x00); // Code-block style

        byte transform = config.Mode == CompressionMode.Lossless ? (byte)0x01 : (byte)0x00;
        stream.WriteByte(transform);
    }

    private void WriteQcdSegment(MemoryStream stream, CompressionConfig config)
    {
        stream.Write(new byte[] { 0xFF, 0x5C }); // QCD marker

        if (config.Mode == CompressionMode.Lossless)
        {
            stream.Write(new byte[] { 0x00, 0x04 }); // Length
            stream.WriteByte(0x22); // Sqcd: reversible
            stream.WriteByte(0x00); // SPqcd
        }
        else
        {
            stream.Write(new byte[] { 0x00, 0x05 }); // Length
            stream.WriteByte(0x42); // Sqcd: scalar derived
            stream.Write(new byte[] { 0x00, 0x88 }); // Base step size
        }
    }

    private byte[] CompressTileData(ImageData image, CompressionConfig config)
    {
        return config.Mode == CompressionMode.Lossless
            ? LosslessEncode(image.PixelData, image.BitsPerSample)
            : LossyEncode(image.PixelData, image.BitsPerSample, config.TargetRatio ?? 10.0f);
    }

    private byte[] LosslessEncode(byte[] data, int bitsPerSample)
    {
        var output = new List<byte>(data.Length);

        if (bitsPerSample <= 8)
        {
            if (data.Length > 0)
            {
                output.Add(data[0]);
                for (int i = 1; i < data.Length; i++)
                {
                    byte delta = (byte)(data[i] - data[i - 1]);
                    output.Add(delta);
                }
            }
        }
        else
        {
            int samples = data.Length / 2;
            if (samples > 0)
            {
                output.Add(data[0]);
                output.Add(data[1]);
                for (int i = 1; i < samples; i++)
                {
                    ushort curr = BitConverter.ToUInt16(data, i * 2);
                    ushort prev = BitConverter.ToUInt16(data, (i - 1) * 2);
                    ushort delta = (ushort)(curr - prev);
                    output.AddRange(BitConverter.GetBytes(delta));
                }
            }
        }

        return output.ToArray();
    }

    private byte[] LossyEncode(byte[] data, int bitsPerSample, float targetRatio)
    {
        int quantBits = Math.Min((int)(Math.Log2(targetRatio) * 0.5), bitsPerSample - 1);
        int shift = quantBits;

        var output = new List<byte> { (byte)quantBits };

        if (bitsPerSample <= 8)
        {
            foreach (byte b in data)
            {
                output.Add((byte)(b >> Math.Min(shift, 7)));
            }
        }
        else
        {
            for (int i = 0; i < data.Length - 1; i += 2)
            {
                ushort value = BitConverter.ToUInt16(data, i);
                ushort quantized = (ushort)(value >> Math.Min(shift, 15));
                output.AddRange(BitConverter.GetBytes(quantized));
            }
        }

        return output.ToArray();
    }

    private byte[] LosslessDecode(byte[] data, int bitsPerSample)
    {
        var output = new List<byte>(data.Length);

        if (bitsPerSample <= 8)
        {
            if (data.Length > 0)
            {
                output.Add(data[0]);
                for (int i = 1; i < data.Length; i++)
                {
                    byte value = (byte)(output[i - 1] + data[i]);
                    output.Add(value);
                }
            }
        }
        else
        {
            if (data.Length >= 2)
            {
                output.Add(data[0]);
                output.Add(data[1]);
                for (int i = 1; i < data.Length / 2; i++)
                {
                    ushort delta = BitConverter.ToUInt16(data, i * 2);
                    ushort prev = BitConverter.ToUInt16(output.ToArray(), (i - 1) * 2);
                    ushort value = (ushort)(prev + delta);
                    output.AddRange(BitConverter.GetBytes(value));
                }
            }
        }

        return output.ToArray();
    }

    private byte[] LossyDecode(byte[] data, int bitsPerSample)
    {
        if (data.Length == 0) return Array.Empty<byte>();

        int quantBits = data[0];
        int shift = Math.Min(quantBits, 15);
        data = data[1..];

        var output = new List<byte>(data.Length << Math.Min(shift, 4));

        if (bitsPerSample <= 8)
        {
            foreach (byte b in data)
            {
                output.Add((byte)(b << Math.Min(shift, 7)));
            }
        }
        else
        {
            for (int i = 0; i < data.Length - 1; i += 2)
            {
                ushort value = BitConverter.ToUInt16(data, i);
                ushort dequantized = (ushort)(value << Math.Min(shift, 15));
                output.AddRange(BitConverter.GetBytes(dequantized));
            }
        }

        return output.ToArray();
    }

    private static void WriteUInt16BE(MemoryStream stream, ushort value)
    {
        stream.WriteByte((byte)(value >> 8));
        stream.WriteByte((byte)(value & 0xFF));
    }

    private static void WriteUInt32BE(MemoryStream stream, uint value)
    {
        stream.WriteByte((byte)(value >> 24));
        stream.WriteByte((byte)((value >> 16) & 0xFF));
        stream.WriteByte((byte)((value >> 8) & 0xFF));
        stream.WriteByte((byte)(value & 0xFF));
    }
}
