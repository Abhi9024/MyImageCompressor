using System.Diagnostics;
using MedImgCompress.Codec;
using MedImgCompress.Config;
using MedImgCompress.Dicom;
using MedImgCompress.Error;

namespace MedImgCompress.Pipeline;

/// <summary>
/// Pipeline for compressing DICOM images.
/// </summary>
public class CompressionPipeline
{
    private readonly CompressionConfig _config;
    private readonly ICodec _codec;

    /// <summary>
    /// Create a new compression pipeline with the specified configuration.
    /// </summary>
    public CompressionPipeline(CompressionConfig config)
    {
        _config = config;
        _codec = CodecFactory.ForConfig(config);
    }

    /// <summary>
    /// Create a pipeline with default configuration.
    /// </summary>
    public static CompressionPipeline Default() =>
        new(CompressionConfig.Default());

    /// <summary>
    /// Create a pipeline for lossless compression.
    /// </summary>
    public static CompressionPipeline Lossless(CompressionCodec codec = CompressionCodec.Jpeg2000) =>
        new(CompressionConfig.Lossless(codec));

    /// <summary>
    /// Create a pipeline for lossy compression.
    /// </summary>
    public static CompressionPipeline Lossy(CompressionCodec codec, float targetRatio) =>
        new(CompressionConfig.Lossy(codec, targetRatio));

    /// <summary>
    /// Compress a DICOM file.
    /// </summary>
    public CompressionResult Compress(string inputPath, string? outputPath = null)
    {
        var stopwatch = Stopwatch.StartNew();

        // Parse input file
        var dicomFile = DicomFile.Open(inputPath);
        var imageData = dicomFile.GetImageData();

        // Validate codec can handle this image
        if (!_codec.CanEncode(imageData))
        {
            throw new PipelineException(
                $"Codec {_codec.Info.Name} cannot encode this image: " +
                $"{imageData.BitsPerSample} bits, {imageData.SamplesPerPixel} samples/pixel");
        }

        // Compress image data
        byte[] compressedPixelData = _codec.Encode(imageData, _config);

        // Get transfer syntax
        bool isLossless = _config.Mode == CompressionMode.Lossless;
        string? transferSyntax = _codec.GetTransferSyntaxUid(isLossless);

        if (string.IsNullOrEmpty(transferSyntax))
        {
            throw new PipelineException(
                $"Codec {_codec.Info.Name} does not support {(isLossless ? "lossless" : "lossy")} compression");
        }

        // Write output file
        var writer = new DicomWriter();
        byte[] outputData = writer.Write(dicomFile, compressedPixelData, transferSyntax);

        if (!string.IsNullOrEmpty(outputPath))
        {
            File.WriteAllBytes(outputPath, outputData);
        }

        stopwatch.Stop();

        return new CompressionResult
        {
            OriginalSize = new FileInfo(inputPath).Length,
            CompressedSize = outputData.Length,
            CodecName = _codec.Info.Name,
            TransferSyntaxUid = transferSyntax,
            IsLossless = isLossless,
            ProcessingTimeMs = stopwatch.ElapsedMilliseconds,
            OutputPath = outputPath,
            CompressedData = string.IsNullOrEmpty(outputPath) ? outputData : null
        };
    }

    /// <summary>
    /// Compress image data directly.
    /// </summary>
    public byte[] CompressImageData(ImageData imageData)
    {
        if (!_codec.CanEncode(imageData))
        {
            throw new PipelineException(
                $"Codec {_codec.Info.Name} cannot encode this image");
        }

        return _codec.Encode(imageData, _config);
    }

    /// <summary>
    /// Decompress image data.
    /// </summary>
    public ImageData Decompress(byte[] compressedData, int width, int height,
        int bitsPerSample, int samplesPerPixel)
    {
        return _codec.Decode(compressedData, width, height, bitsPerSample, samplesPerPixel);
    }

    /// <summary>
    /// Get information about the configured codec.
    /// </summary>
    public CodecInfo GetCodecInfo() => _codec.Info;

    /// <summary>
    /// Get the capabilities of the configured codec.
    /// </summary>
    public CodecCapabilities GetCodecCapabilities() => _codec.Capabilities;
}
