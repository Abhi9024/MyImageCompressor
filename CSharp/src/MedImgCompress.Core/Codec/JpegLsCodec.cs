using MedImgCompress.Config;
using MedImgCompress.Error;

namespace MedImgCompress.Codec;

/// <summary>
/// JPEG-LS codec implementation.
/// </summary>
public class JpegLsCodec : ICodec
{
    /// <summary>
    /// Maximum near-lossless error tolerance (0 = lossless).
    /// </summary>
    public byte Near { get; set; } = 0;

    public CodecInfo Info => new()
    {
        Name = "JPEG-LS",
        Version = "MVP 0.1",
        SupportsLossless = true,
        SupportsLossy = true,
        SupportsProgressive = false,
        SupportsRoi = false,
        TransferSyntaxLossless = TransferSyntax.JpegLsLossless,
        TransferSyntaxLossy = TransferSyntax.JpegLsNearLossless
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
    public static JpegLsCodec Lossless() => new() { Near = 0 };

    /// <summary>
    /// Create codec configured for near-lossless compression.
    /// </summary>
    public static JpegLsCodec NearLossless(byte tolerance) => new() { Near = tolerance };

    public byte[] Encode(ImageData image, CompressionConfig config)
    {
        ValidateImage(image);

        byte near = config.Mode == CompressionMode.NearLossless ? config.NearLosslessError : (byte)0;

        using var stream = new MemoryStream();

        // SOI marker
        stream.Write(new byte[] { 0xFF, 0xD8 });

        // SOF55 segment
        WriteSof55Segment(stream, image);

        // LSE segment if near-lossless
        if (near > 0)
        {
            WriteLseSegment(stream);
        }

        // SOS segment
        WriteSosSegment(stream, image, near);

        // Compressed data
        byte[] compressed = CompressData(image, near);
        stream.Write(compressed);

        // EOI marker
        stream.Write(new byte[] { 0xFF, 0xD9 });

        return stream.ToArray();
    }

    public ImageData Decode(byte[] data, int width, int height, int bitsPerSample, int samplesPerPixel)
    {
        if (data.Length < 4)
            throw new CodecException("Invalid JPEG-LS data: too short");

        if (data[0] != 0xFF || data[1] != 0xD8)
            throw new CodecException("Invalid JPEG-LS data: missing SOI marker");

        // Parse header to find NEAR parameter and data start
        var (near, dataStart) = ParseHeader(data);

        // Find EOI marker
        int dataEnd = data.Length >= 2 && data[^2] == 0xFF && data[^1] == 0xD9
            ? data.Length - 2
            : data.Length;

        if (dataStart >= dataEnd)
            throw new CodecException("Invalid JPEG-LS data: no image data");

        byte[] compressed = new byte[dataEnd - dataStart];
        Array.Copy(data, dataStart, compressed, 0, compressed.Length);

        int bytesPerSample = (bitsPerSample + 7) / 8;
        byte[] decompressed = bytesPerSample == 1
            ? Decompress8Bit(compressed, width, height, near)
            : Decompress16Bit(compressed, width, height, near);

        return new ImageData
        {
            Width = width,
            Height = height,
            BitsPerSample = bitsPerSample,
            SamplesPerPixel = samplesPerPixel,
            PixelData = decompressed,
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
    }

    private void WriteSof55Segment(MemoryStream stream, ImageData image)
    {
        stream.Write(new byte[] { 0xFF, 0xF7 }); // SOF55 marker

        int length = 8 + 3 * image.SamplesPerPixel;
        WriteUInt16BE(stream, (ushort)length);

        stream.WriteByte((byte)image.BitsPerSample);
        WriteUInt16BE(stream, (ushort)image.Height);
        WriteUInt16BE(stream, (ushort)image.Width);
        stream.WriteByte((byte)image.SamplesPerPixel);

        for (int i = 0; i < image.SamplesPerPixel; i++)
        {
            stream.WriteByte((byte)(i + 1)); // Component ID
            stream.WriteByte(0x11);           // Sampling factors
            stream.WriteByte(0x00);           // Quantization table
        }
    }

    private void WriteLseSegment(MemoryStream stream)
    {
        stream.Write(new byte[] { 0xFF, 0xF8 }); // LSE marker
        stream.Write(new byte[] { 0x00, 0x0D }); // Length
        stream.WriteByte(0x01); // ID

        // MAXVAL, T1, T2, T3, RESET
        stream.Write(new byte[] { 0x00, 0xFF }); // MAXVAL
        stream.Write(new byte[] { 0x00, 0x03 }); // T1
        stream.Write(new byte[] { 0x00, 0x07 }); // T2
        stream.Write(new byte[] { 0x00, 0x15 }); // T3
        stream.Write(new byte[] { 0x00, 0x40 }); // RESET
    }

    private void WriteSosSegment(MemoryStream stream, ImageData image, byte near)
    {
        stream.Write(new byte[] { 0xFF, 0xDA }); // SOS marker

        int length = 6 + 2 * image.SamplesPerPixel;
        WriteUInt16BE(stream, (ushort)length);

        stream.WriteByte((byte)image.SamplesPerPixel);

        for (int i = 0; i < image.SamplesPerPixel; i++)
        {
            stream.WriteByte((byte)(i + 1)); // Component ID
            stream.WriteByte(0x00);           // Mapping table
        }

        stream.WriteByte(near); // NEAR parameter
        stream.WriteByte(image.SamplesPerPixel > 1 ? (byte)2 : (byte)0); // Interleave mode
        stream.WriteByte(0x00); // Point transform
    }

    private byte[] CompressData(ImageData image, byte near)
    {
        int bytesPerSample = (image.BitsPerSample + 7) / 8;
        return bytesPerSample == 1
            ? Compress8Bit(image.PixelData, image.Width, near)
            : Compress16Bit(image.PixelData, image.Width, near);
    }

    private byte[] Compress8Bit(byte[] data, int width, byte near)
    {
        int height = data.Length / width;
        var output = new List<byte>(data.Length);

        for (int y = 0; y < height; y++)
        {
            for (int x = 0; x < width; x++)
            {
                int idx = y * width + x;
                byte current = data[idx];

                byte prediction = GetPrediction8Bit(data, width, x, y);
                byte error = (byte)(current - prediction);

                byte quantizedError = near > 0
                    ? (byte)(((sbyte)error + near) / (2 * near + 1))
                    : error;

                output.Add(quantizedError);
            }
        }

        return output.ToArray();
    }

    private byte[] Compress16Bit(byte[] data, int width, byte near)
    {
        int samples = data.Length / 2;
        int height = samples / width;
        var output = new List<byte>(data.Length);

        for (int y = 0; y < height; y++)
        {
            for (int x = 0; x < width; x++)
            {
                int idx = y * width + x;
                ushort current = BitConverter.ToUInt16(data, idx * 2);

                ushort prediction = GetPrediction16Bit(data, width, x, y);
                ushort error = (ushort)(current - prediction);

                if (near > 0)
                {
                    int n = near * 256;
                    short q = (short)(((short)error + n) / (2 * n + 1));
                    error = (ushort)q;
                }

                output.AddRange(BitConverter.GetBytes(error));
            }
        }

        return output.ToArray();
    }

    private byte GetPrediction8Bit(byte[] data, int width, int x, int y)
    {
        if (x == 0 && y == 0) return 128;
        if (y == 0) return data[y * width + x - 1];
        if (x == 0) return data[(y - 1) * width + x];

        int a = data[y * width + x - 1];
        int b = data[(y - 1) * width + x];
        int c = data[(y - 1) * width + x - 1];

        if (c >= Math.Max(a, b)) return (byte)Math.Min(a, b);
        if (c <= Math.Min(a, b)) return (byte)Math.Max(a, b);
        return (byte)Math.Clamp(a + b - c, 0, 255);
    }

    private ushort GetPrediction16Bit(byte[] data, int width, int x, int y)
    {
        if (x == 0 && y == 0) return 32768;

        int idx = y * width + x;
        if (y == 0) return BitConverter.ToUInt16(data, (idx - 1) * 2);
        if (x == 0) return BitConverter.ToUInt16(data, (idx - width) * 2);

        int a = BitConverter.ToUInt16(data, (idx - 1) * 2);
        int b = BitConverter.ToUInt16(data, (idx - width) * 2);
        int c = BitConverter.ToUInt16(data, (idx - width - 1) * 2);

        if (c >= Math.Max(a, b)) return (ushort)Math.Min(a, b);
        if (c <= Math.Min(a, b)) return (ushort)Math.Max(a, b);
        return (ushort)Math.Clamp(a + b - c, 0, 65535);
    }

    private (byte near, int dataStart) ParseHeader(byte[] data)
    {
        int pos = 2;
        byte near = 0;

        while (pos < data.Length - 1)
        {
            if (data[pos] != 0xFF)
            {
                pos++;
                continue;
            }

            byte marker = data[pos + 1];
            pos += 2;

            if (marker == 0xDA) // SOS
            {
                if (pos + 2 > data.Length) break;

                int length = (data[pos] << 8) | data[pos + 1];
                if (pos + length > data.Length) break;

                int numComponents = data[pos + 2];
                int nearOffset = pos + 3 + 2 * numComponents;
                if (nearOffset < data.Length)
                {
                    near = data[nearOffset];
                }

                return (near, pos + length);
            }
            else if (marker == 0xD9) // EOI
            {
                break;
            }
            else if (marker == 0x00) // Stuffed byte
            {
                continue;
            }
            else
            {
                if (pos + 2 <= data.Length)
                {
                    int length = (data[pos] << 8) | data[pos + 1];
                    pos += length;
                }
            }
        }

        throw new CodecException("Could not find SOS marker in JPEG-LS data");
    }

    private byte[] Decompress8Bit(byte[] data, int width, int height, byte near)
    {
        byte[] output = new byte[width * height];

        for (int y = 0; y < height; y++)
        {
            for (int x = 0; x < width; x++)
            {
                int idx = y * width + x;
                if (idx >= data.Length) break;

                byte error = data[idx];
                byte prediction = GetPrediction8BitOutput(output, width, x, y);

                byte dequantizedError = near > 0
                    ? (byte)((sbyte)error * (2 * near + 1))
                    : error;

                output[idx] = (byte)(prediction + dequantizedError);
            }
        }

        return output;
    }

    private byte[] Decompress16Bit(byte[] data, int width, int height, byte near)
    {
        byte[] output = new byte[width * height * 2];
        int samples = width * height;

        for (int i = 0; i < samples; i++)
        {
            int y = i / width;
            int x = i % width;

            if (i * 2 + 1 >= data.Length) break;

            ushort error = BitConverter.ToUInt16(data, i * 2);
            ushort prediction = GetPrediction16BitOutput(output, width, x, y);

            if (near > 0)
            {
                int n = near * 256;
                error = (ushort)((short)error * (2 * n + 1));
            }

            ushort value = (ushort)(prediction + error);
            byte[] valueBytes = BitConverter.GetBytes(value);
            output[i * 2] = valueBytes[0];
            output[i * 2 + 1] = valueBytes[1];
        }

        return output;
    }

    private byte GetPrediction8BitOutput(byte[] output, int width, int x, int y)
    {
        if (x == 0 && y == 0) return 128;
        if (y == 0) return output[y * width + x - 1];
        if (x == 0) return output[(y - 1) * width + x];

        int a = output[y * width + x - 1];
        int b = output[(y - 1) * width + x];
        int c = output[(y - 1) * width + x - 1];

        if (c >= Math.Max(a, b)) return (byte)Math.Min(a, b);
        if (c <= Math.Min(a, b)) return (byte)Math.Max(a, b);
        return (byte)Math.Clamp(a + b - c, 0, 255);
    }

    private ushort GetPrediction16BitOutput(byte[] output, int width, int x, int y)
    {
        int idx = y * width + x;
        if (x == 0 && y == 0) return 32768;
        if (y == 0) return BitConverter.ToUInt16(output, (idx - 1) * 2);
        if (x == 0) return BitConverter.ToUInt16(output, (idx - width) * 2);

        int a = BitConverter.ToUInt16(output, (idx - 1) * 2);
        int b = BitConverter.ToUInt16(output, (idx - width) * 2);
        int c = BitConverter.ToUInt16(output, (idx - width - 1) * 2);

        if (c >= Math.Max(a, b)) return (ushort)Math.Min(a, b);
        if (c <= Math.Min(a, b)) return (ushort)Math.Max(a, b);
        return (ushort)Math.Clamp(a + b - c, 0, 65535);
    }

    private static void WriteUInt16BE(MemoryStream stream, ushort value)
    {
        stream.WriteByte((byte)(value >> 8));
        stream.WriteByte((byte)(value & 0xFF));
    }
}
