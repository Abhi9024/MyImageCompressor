namespace MedImgCompress.Error;

/// <summary>
/// Base exception for medical image compression errors.
/// </summary>
public class MedImgException : Exception
{
    public MedImgException(string message) : base(message) { }
    public MedImgException(string message, Exception innerException) : base(message, innerException) { }
}

/// <summary>
/// Exception for DICOM parsing errors.
/// </summary>
public class DicomException : MedImgException
{
    public DicomException(string message) : base($"DICOM error: {message}") { }
    public DicomException(string message, Exception innerException) : base($"DICOM error: {message}", innerException) { }
}

/// <summary>
/// Exception for codec errors.
/// </summary>
public class CodecException : MedImgException
{
    public CodecException(string message) : base($"Codec error: {message}") { }
    public CodecException(string message, Exception innerException) : base($"Codec error: {message}", innerException) { }
}

/// <summary>
/// Exception for invalid image format.
/// </summary>
public class InvalidFormatException : MedImgException
{
    public InvalidFormatException(string message) : base($"Invalid format: {message}") { }
}

/// <summary>
/// Exception for unsupported transfer syntax.
/// </summary>
public class UnsupportedTransferSyntaxException : MedImgException
{
    public string TransferSyntax { get; }

    public UnsupportedTransferSyntaxException(string transferSyntax)
        : base($"Unsupported transfer syntax: {transferSyntax}")
    {
        TransferSyntax = transferSyntax;
    }
}

/// <summary>
/// Exception for configuration errors.
/// </summary>
public class ConfigurationException : MedImgException
{
    public ConfigurationException(string message) : base($"Configuration error: {message}") { }
}

/// <summary>
/// Exception for validation errors (e.g., regulatory constraints).
/// </summary>
public class ValidationException : MedImgException
{
    public ValidationException(string message) : base($"Validation error: {message}") { }
}

/// <summary>
/// Exception for image data errors.
/// </summary>
public class ImageDataException : MedImgException
{
    public ImageDataException(string message) : base($"Image data error: {message}") { }
}

/// <summary>
/// Exception for compression constraint violations.
/// </summary>
public class CompressionConstraintException : MedImgException
{
    public CompressionConstraintException(string message) : base($"Compression constraint violation: {message}") { }
}

/// <summary>
/// Exception for pipeline processing errors.
/// </summary>
public class PipelineException : MedImgException
{
    public PipelineException(string message) : base($"Pipeline error: {message}") { }
    public PipelineException(string message, Exception innerException) : base($"Pipeline error: {message}", innerException) { }
}
