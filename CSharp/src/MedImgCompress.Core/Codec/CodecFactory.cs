using MedImgCompress.Config;

namespace MedImgCompress.Codec;

/// <summary>
/// Factory for creating codec instances.
/// </summary>
public static class CodecFactory
{
    /// <summary>
    /// Create a codec instance based on codec type.
    /// </summary>
    public static ICodec Create(CompressionCodec codecType)
    {
        return codecType switch
        {
            CompressionCodec.Jpeg2000 => new Jpeg2000Codec(),
            CompressionCodec.JpegLs => new JpegLsCodec(),
            CompressionCodec.Uncompressed => new UncompressedCodec(),
            _ => throw new ArgumentOutOfRangeException(nameof(codecType), codecType, "Unknown codec type")
        };
    }

    /// <summary>
    /// Get the appropriate codec for the given configuration.
    /// </summary>
    public static ICodec ForConfig(CompressionConfig config)
    {
        return Create(config.Codec);
    }
}

/// <summary>
/// Passthrough codec for uncompressed data.
/// </summary>
internal class UncompressedCodec : ICodec
{
    public CodecInfo Info => new()
    {
        Name = "Uncompressed",
        Version = "1.0",
        SupportsLossless = true,
        SupportsLossy = false,
        SupportsProgressive = false,
        SupportsRoi = false,
        TransferSyntaxLossless = TransferSyntax.ExplicitVrLittleEndian,
        TransferSyntaxLossy = null
    };

    public CodecCapabilities Capabilities => new()
    {
        MaxBitsPerSample = 16,
        SupportsSigned = true,
        SupportsColor = true,
        SupportsMultiframe = true
    };

    public byte[] Encode(ImageData image, CompressionConfig config)
    {
        return image.PixelData.ToArray();
    }

    public ImageData Decode(byte[] data, int width, int height, int bitsPerSample, int samplesPerPixel)
    {
        return new ImageData
        {
            Width = width,
            Height = height,
            BitsPerSample = bitsPerSample,
            SamplesPerPixel = samplesPerPixel,
            PixelData = data.ToArray(),
            PhotometricInterpretation = string.Empty,
            IsSigned = false
        };
    }
}
