using MedImgCompress.Config;

namespace MedImgCompress.Codec;

/// <summary>
/// Information about a codec.
/// </summary>
public record CodecInfo
{
    /// <summary>Human-readable codec name.</summary>
    public required string Name { get; init; }

    /// <summary>Codec version string.</summary>
    public required string Version { get; init; }

    /// <summary>Whether lossless compression is supported.</summary>
    public bool SupportsLossless { get; init; }

    /// <summary>Whether lossy compression is supported.</summary>
    public bool SupportsLossy { get; init; }

    /// <summary>Whether progressive/multi-resolution decoding is supported.</summary>
    public bool SupportsProgressive { get; init; }

    /// <summary>Whether ROI (Region of Interest) encoding is supported.</summary>
    public bool SupportsRoi { get; init; }

    /// <summary>DICOM Transfer Syntax UID for lossless mode.</summary>
    public string? TransferSyntaxLossless { get; init; }

    /// <summary>DICOM Transfer Syntax UID for lossy mode.</summary>
    public string? TransferSyntaxLossy { get; init; }
}

/// <summary>
/// Codec capabilities for image formats.
/// </summary>
public record CodecCapabilities
{
    /// <summary>Maximum supported bits per sample.</summary>
    public int MaxBitsPerSample { get; init; }

    /// <summary>Whether signed pixel values are supported.</summary>
    public bool SupportsSigned { get; init; }

    /// <summary>Whether color images are supported.</summary>
    public bool SupportsColor { get; init; }

    /// <summary>Whether multi-frame images are supported.</summary>
    public bool SupportsMultiframe { get; init; }
}

/// <summary>
/// Interface for image compression/decompression codecs.
/// </summary>
public interface ICodec
{
    /// <summary>
    /// Encode image data to compressed format.
    /// </summary>
    /// <param name="image">The image data to compress.</param>
    /// <param name="config">Compression configuration.</param>
    /// <returns>Compressed data as bytes.</returns>
    byte[] Encode(ImageData image, CompressionConfig config);

    /// <summary>
    /// Decode compressed data to image.
    /// </summary>
    /// <param name="data">Compressed image data.</param>
    /// <param name="width">Image width in pixels.</param>
    /// <param name="height">Image height in pixels.</param>
    /// <param name="bitsPerSample">Bits per pixel sample.</param>
    /// <param name="samplesPerPixel">Number of samples per pixel.</param>
    /// <returns>Decoded image data.</returns>
    ImageData Decode(byte[] data, int width, int height, int bitsPerSample, int samplesPerPixel);

    /// <summary>
    /// Get codec information.
    /// </summary>
    CodecInfo Info { get; }

    /// <summary>
    /// Get codec capabilities.
    /// </summary>
    CodecCapabilities Capabilities { get; }

    /// <summary>
    /// Verify that the codec can handle the given image.
    /// </summary>
    bool CanEncode(ImageData image)
    {
        var caps = Capabilities;
        return image.BitsPerSample <= caps.MaxBitsPerSample
            && (image.SamplesPerPixel == 1 || caps.SupportsColor)
            && (!image.IsSigned || caps.SupportsSigned);
    }

    /// <summary>
    /// Get the DICOM transfer syntax UID for the given compression mode.
    /// </summary>
    string? GetTransferSyntaxUid(bool lossless)
    {
        return lossless ? Info.TransferSyntaxLossless : Info.TransferSyntaxLossy;
    }
}
