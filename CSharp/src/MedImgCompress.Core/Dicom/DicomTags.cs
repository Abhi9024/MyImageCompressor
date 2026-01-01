namespace MedImgCompress.Dicom;

/// <summary>
/// Common DICOM tag definitions.
/// </summary>
public static class DicomTags
{
    // Patient Module
    public const uint PatientName = 0x00100010;
    public const uint PatientId = 0x00100020;
    public const uint PatientBirthDate = 0x00100030;
    public const uint PatientSex = 0x00100040;

    // Study Module
    public const uint StudyInstanceUid = 0x0020000D;
    public const uint StudyDate = 0x00080020;
    public const uint StudyTime = 0x00080030;
    public const uint StudyDescription = 0x00081030;

    // Series Module
    public const uint SeriesInstanceUid = 0x0020000E;
    public const uint Modality = 0x00080060;
    public const uint SeriesNumber = 0x00200011;

    // Image Module
    public const uint SopInstanceUid = 0x00080018;
    public const uint SopClassUid = 0x00080016;
    public const uint InstanceNumber = 0x00200013;

    // Image Pixel Module
    public const uint Rows = 0x00280010;
    public const uint Columns = 0x00280011;
    public const uint BitsAllocated = 0x00280100;
    public const uint BitsStored = 0x00280101;
    public const uint HighBit = 0x00280102;
    public const uint PixelRepresentation = 0x00280103;
    public const uint SamplesPerPixel = 0x00280002;
    public const uint PhotometricInterpretation = 0x00280004;
    public const uint PlanarConfiguration = 0x00280006;
    public const uint PixelData = 0x7FE00010;

    // Transfer Syntax
    public const uint TransferSyntaxUid = 0x00020010;

    /// <summary>
    /// Get the group number from a tag.
    /// </summary>
    public static ushort GetGroup(uint tag) => (ushort)(tag >> 16);

    /// <summary>
    /// Get the element number from a tag.
    /// </summary>
    public static ushort GetElement(uint tag) => (ushort)(tag & 0xFFFF);

    /// <summary>
    /// Create a tag from group and element numbers.
    /// </summary>
    public static uint MakeTag(ushort group, ushort element) =>
        ((uint)group << 16) | element;
}
