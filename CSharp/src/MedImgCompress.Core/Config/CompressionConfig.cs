using MedImgCompress.Error;

namespace MedImgCompress.Config;

/// <summary>
/// Configuration for compression operation.
/// </summary>
public class CompressionConfig
{
    /// <summary>Codec to use for compression.</summary>
    public CompressionCodec Codec { get; set; } = CompressionCodec.Jpeg2000;

    /// <summary>Compression mode (lossless, lossy, near-lossless).</summary>
    public CompressionMode Mode { get; set; } = CompressionMode.Lossless;

    /// <summary>Quality preset.</summary>
    public QualityPreset Quality { get; set; } = QualityPreset.Diagnostic;

    /// <summary>Target compression ratio (for lossy mode).</summary>
    public float? TargetRatio { get; set; }

    /// <summary>JPEG 2000 specific: number of quality layers.</summary>
    public int QualityLayers { get; set; } = 1;

    /// <summary>JPEG 2000 specific: tile size (0 = no tiling).</summary>
    public int TileSize { get; set; } = 0;

    /// <summary>JPEG-LS specific: near-lossless tolerance (0 = lossless).</summary>
    public byte NearLosslessError { get; set; } = 0;

    /// <summary>Preserve original DICOM metadata exactly.</summary>
    public bool PreserveMetadata { get; set; } = true;

    /// <summary>Verify compression by round-trip decode.</summary>
    public bool VerifyCompression { get; set; } = true;

    /// <summary>Override modality safety checks (use with caution).</summary>
    public bool OverrideSafetyChecks { get; set; } = false;

    /// <summary>
    /// Create a lossless configuration with the given codec.
    /// </summary>
    public static CompressionConfig Lossless(CompressionCodec codec = CompressionCodec.Jpeg2000)
    {
        return new CompressionConfig
        {
            Codec = codec,
            Mode = CompressionMode.Lossless,
            Quality = QualityPreset.Diagnostic
        };
    }

    /// <summary>
    /// Create a lossy configuration with target ratio.
    /// </summary>
    public static CompressionConfig Lossy(CompressionCodec codec, float ratio)
    {
        return new CompressionConfig
        {
            Codec = codec,
            Mode = CompressionMode.Lossy,
            Quality = QualityPreset.Standard,
            TargetRatio = ratio
        };
    }

    /// <summary>
    /// Validate configuration against modality constraints.
    /// </summary>
    /// <exception cref="ValidationException">Thrown when validation fails.</exception>
    public void ValidateForModality(Modality modality)
    {
        if (modality.RequiresLossless() && Mode != CompressionMode.Lossless)
        {
            if (OverrideSafetyChecks)
            {
                Console.WriteLine($"Warning: Safety check overridden: {modality} typically requires lossless compression");
            }
            else
            {
                throw new ValidationException(
                    $"Modality {modality} requires lossless compression (FDA/ACR requirement). " +
                    "Set OverrideSafetyChecks=true to bypass.");
            }
        }
    }
}

/// <summary>
/// Transfer syntax UIDs for DICOM.
/// </summary>
public static class TransferSyntax
{
    /// <summary>JPEG 2000 Lossless</summary>
    public const string Jpeg2000Lossless = "1.2.840.10008.1.2.4.90";

    /// <summary>JPEG 2000 Lossy</summary>
    public const string Jpeg2000Lossy = "1.2.840.10008.1.2.4.91";

    /// <summary>JPEG-LS Lossless</summary>
    public const string JpegLsLossless = "1.2.840.10008.1.2.4.80";

    /// <summary>JPEG-LS Near-Lossless</summary>
    public const string JpegLsNearLossless = "1.2.840.10008.1.2.4.81";

    /// <summary>Explicit VR Little Endian (uncompressed)</summary>
    public const string ExplicitVrLittleEndian = "1.2.840.10008.1.2.1";

    /// <summary>Implicit VR Little Endian (uncompressed)</summary>
    public const string ImplicitVrLittleEndian = "1.2.840.10008.1.2";

    /// <summary>
    /// Check if transfer syntax is lossless.
    /// </summary>
    public static bool IsLossless(string transferSyntax)
    {
        return transferSyntax switch
        {
            ImplicitVrLittleEndian => true,
            ExplicitVrLittleEndian => true,
            "1.2.840.10008.1.2.2" => true,  // Explicit VR Big Endian
            "1.2.840.10008.1.2.4.70" => true,  // JPEG Lossless
            JpegLsLossless => true,
            Jpeg2000Lossless => true,
            "1.2.840.10008.1.2.5" => true,  // RLE Lossless
            _ => false
        };
    }

    /// <summary>
    /// Get human-readable name for transfer syntax.
    /// </summary>
    public static string GetName(string transferSyntax)
    {
        return transferSyntax switch
        {
            ImplicitVrLittleEndian => "Implicit VR Little Endian",
            ExplicitVrLittleEndian => "Explicit VR Little Endian",
            "1.2.840.10008.1.2.2" => "Explicit VR Big Endian",
            "1.2.840.10008.1.2.4.70" => "JPEG Lossless",
            JpegLsLossless => "JPEG-LS Lossless",
            JpegLsNearLossless => "JPEG-LS Near-Lossless",
            Jpeg2000Lossless => "JPEG 2000 Lossless",
            Jpeg2000Lossy => "JPEG 2000 Lossy",
            "1.2.840.10008.1.2.5" => "RLE Lossless",
            _ => "Unknown"
        };
    }
}
