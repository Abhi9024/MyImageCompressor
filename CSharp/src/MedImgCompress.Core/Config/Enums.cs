namespace MedImgCompress.Config;

/// <summary>
/// Supported compression codecs.
/// </summary>
public enum CompressionCodec
{
    /// <summary>JPEG 2000 (lossless or lossy)</summary>
    Jpeg2000,

    /// <summary>JPEG-LS (lossless or near-lossless)</summary>
    JpegLs,

    /// <summary>No compression (raw)</summary>
    Uncompressed
}

/// <summary>
/// Compression mode.
/// </summary>
public enum CompressionMode
{
    /// <summary>Lossless compression - exact reconstruction guaranteed.</summary>
    Lossless,

    /// <summary>Lossy compression with quality parameter.</summary>
    Lossy,

    /// <summary>Near-lossless (JPEG-LS only) with maximum error tolerance.</summary>
    NearLossless
}

/// <summary>
/// Medical imaging modality.
/// </summary>
public enum Modality
{
    /// <summary>Computed Tomography</summary>
    CT,

    /// <summary>Magnetic Resonance Imaging</summary>
    MR,

    /// <summary>Computed/Digital Radiography</summary>
    CR,

    /// <summary>Digital X-Ray</summary>
    DX,

    /// <summary>Mammography - requires lossless only</summary>
    MG,

    /// <summary>Ultrasound</summary>
    US,

    /// <summary>Nuclear Medicine</summary>
    NM,

    /// <summary>Positron Emission Tomography</summary>
    PT,

    /// <summary>Whole Slide Imaging (Pathology)</summary>
    SM,

    /// <summary>Other/Unknown</summary>
    Other
}

/// <summary>
/// Quality preset for compression.
/// </summary>
public enum QualityPreset
{
    /// <summary>Maximum quality - lossless</summary>
    Diagnostic,

    /// <summary>High quality lossy - suitable for primary review</summary>
    HighQuality,

    /// <summary>Medium quality - suitable for reference viewing</summary>
    Standard,

    /// <summary>Lower quality - thumbnails and previews</summary>
    Preview
}

/// <summary>
/// Extension methods for Modality enum.
/// </summary>
public static class ModalityExtensions
{
    /// <summary>
    /// Parse modality from DICOM modality code.
    /// </summary>
    public static Modality FromDicomCode(string? modalityString)
    {
        if (string.IsNullOrWhiteSpace(modalityString))
            return Modality.Other;

        return modalityString.Trim().ToUpperInvariant() switch
        {
            "CT" => Modality.CT,
            "MR" or "MRI" => Modality.MR,
            "CR" => Modality.CR,
            "DX" => Modality.DX,
            "MG" => Modality.MG,
            "US" => Modality.US,
            "NM" => Modality.NM,
            "PT" or "PET" => Modality.PT,
            "SM" => Modality.SM,
            _ => Modality.Other
        };
    }

    /// <summary>
    /// Check if modality requires lossless compression (regulatory requirement).
    /// </summary>
    public static bool RequiresLossless(this Modality modality)
    {
        return modality == Modality.MG;
    }

    /// <summary>
    /// Get recommended codec for this modality.
    /// </summary>
    public static CompressionCodec RecommendedCodec(this Modality modality)
    {
        return modality switch
        {
            Modality.NM => CompressionCodec.JpegLs,  // Lower resolution, fast
            _ => CompressionCodec.Jpeg2000           // General recommendation
        };
    }

    /// <summary>
    /// Get default quality preset for this modality.
    /// </summary>
    public static QualityPreset GetDefaultPreset(this Modality modality)
    {
        return modality switch
        {
            Modality.MG => QualityPreset.Diagnostic,  // Mammography requires lossless
            Modality.CT => QualityPreset.HighQuality,
            Modality.MR => QualityPreset.HighQuality,
            Modality.SM => QualityPreset.HighQuality, // Pathology needs high detail
            _ => QualityPreset.Standard
        };
    }
}

/// <summary>
/// Extension methods for QualityPreset enum.
/// </summary>
public static class QualityPresetExtensions
{
    /// <summary>
    /// Get the compression ratio target for lossy compression.
    /// </summary>
    public static float? TargetRatio(this QualityPreset preset)
    {
        return preset switch
        {
            QualityPreset.Diagnostic => null,  // Lossless
            QualityPreset.HighQuality => 10.0f,
            QualityPreset.Standard => 20.0f,
            QualityPreset.Preview => 50.0f,
            _ => null
        };
    }

    /// <summary>
    /// Get JPEG 2000 quality layers.
    /// </summary>
    public static int QualityLayers(this QualityPreset preset)
    {
        return preset switch
        {
            QualityPreset.Diagnostic => 1,
            QualityPreset.HighQuality => 5,
            QualityPreset.Standard => 3,
            QualityPreset.Preview => 2,
            _ => 1
        };
    }
}
