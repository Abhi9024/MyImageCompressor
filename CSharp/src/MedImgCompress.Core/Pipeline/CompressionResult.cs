namespace MedImgCompress.Pipeline;

/// <summary>
/// Result of a compression operation.
/// </summary>
public class CompressionResult
{
    /// <summary>
    /// Original file size in bytes.
    /// </summary>
    public long OriginalSize { get; init; }

    /// <summary>
    /// Compressed file size in bytes.
    /// </summary>
    public long CompressedSize { get; init; }

    /// <summary>
    /// Compression ratio (original / compressed).
    /// </summary>
    public double CompressionRatio => OriginalSize > 0
        ? (double)OriginalSize / CompressedSize
        : 0;

    /// <summary>
    /// Space savings as percentage.
    /// </summary>
    public double SpaceSavingsPercent => OriginalSize > 0
        ? (1.0 - (double)CompressedSize / OriginalSize) * 100
        : 0;

    /// <summary>
    /// Codec used for compression.
    /// </summary>
    public required string CodecName { get; init; }

    /// <summary>
    /// Transfer syntax UID used.
    /// </summary>
    public required string TransferSyntaxUid { get; init; }

    /// <summary>
    /// Whether lossless compression was used.
    /// </summary>
    public bool IsLossless { get; init; }

    /// <summary>
    /// Processing time in milliseconds.
    /// </summary>
    public long ProcessingTimeMs { get; init; }

    /// <summary>
    /// Output file path (if written to file).
    /// </summary>
    public string? OutputPath { get; init; }

    /// <summary>
    /// Compressed data (if kept in memory).
    /// </summary>
    public byte[]? CompressedData { get; init; }

    /// <summary>
    /// Create a summary string.
    /// </summary>
    public override string ToString()
    {
        return $"Compression: {CodecName} ({(IsLossless ? "lossless" : "lossy")})\n" +
               $"Original: {FormatSize(OriginalSize)}\n" +
               $"Compressed: {FormatSize(CompressedSize)}\n" +
               $"Ratio: {CompressionRatio:F2}:1 ({SpaceSavingsPercent:F1}% savings)\n" +
               $"Time: {ProcessingTimeMs}ms";
    }

    private static string FormatSize(long bytes)
    {
        string[] units = { "B", "KB", "MB", "GB" };
        double size = bytes;
        int unit = 0;

        while (size >= 1024 && unit < units.Length - 1)
        {
            size /= 1024;
            unit++;
        }

        return $"{size:F2} {units[unit]}";
    }
}
