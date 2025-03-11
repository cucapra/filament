//! Tracking of source positions
use codespan_reporting::{diagnostic::Label, files};
use std::{ops::Range, sync};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
/// Handle to a position in a [PositionTable]
/// The index refers to the index in the [PositionTable::indices] vector.
pub struct PosIdx(u32);

#[derive(Clone, Copy, PartialEq, Eq)]
/// Handle to a file in a [PositionTable]
/// The index refers to the index in the [PositionTable::files] vector.
pub struct FileIdx(usize);

impl FileIdx {
    /// Get the index of the file
    pub fn get(self) -> usize {
        self.0
    }
}

/// The data associated with a position
pub struct PosData {
    /// The file in the program. The index refers to the index in the
    /// [PositionTable::files] vector.
    pub file: FileIdx,
    /// Start of the span
    pub start: usize,
    /// End of the span
    pub end: usize,
}

/// Reimplementation of [files::SimpleFiles] that uses
/// [boxcar::Vec] to avoid mutable borrows.
#[derive(Clone, Debug, Default)]
pub struct BoxcarSimpleFiles<Name, Source> {
    files: boxcar::Vec<files::SimpleFile<Name, Source>>,
}

impl<Name, Source> BoxcarSimpleFiles<Name, Source>
where
    Name: std::fmt::Display,
    Source: AsRef<str>,
{
    /// Create a new files database.
    pub fn new() -> BoxcarSimpleFiles<Name, Source> {
        BoxcarSimpleFiles {
            files: boxcar::Vec::new(),
        }
    }

    /// Add a file to the database, returning the handle that can be used to
    /// refer to it again.
    pub fn add(&self, name: Name, source: Source) -> usize {
        let file_id = self.files.count();
        self.files.push(files::SimpleFile::new(name, source));
        file_id
    }

    /// Get the file corresponding to the given id.
    pub fn get(
        &self,
        file_id: usize,
    ) -> Result<&files::SimpleFile<Name, Source>, files::Error> {
        self.files.get(file_id).ok_or(files::Error::FileMissing)
    }
}

impl<'a, Name, Source> files::Files<'a> for BoxcarSimpleFiles<Name, Source>
where
    Name: 'a + std::fmt::Display + Clone,
    Source: 'a + AsRef<str>,
{
    type FileId = usize;
    type Name = Name;
    type Source = &'a str;

    fn name(&self, file_id: usize) -> Result<Name, files::Error> {
        Ok(self.get(file_id)?.name().clone())
    }

    fn source(&self, file_id: usize) -> Result<&str, files::Error> {
        Ok(self.get(file_id)?.source().as_ref())
    }

    fn line_index(
        &self,
        file_id: usize,
        byte_index: usize,
    ) -> Result<usize, files::Error> {
        self.get(file_id)?.line_index((), byte_index)
    }

    fn line_range(
        &self,
        file_id: usize,
        line_index: usize,
    ) -> Result<Range<usize>, files::Error> {
        self.get(file_id)?.line_range((), line_index)
    }
}

/// Source position information for a full program
pub struct PositionTable {
    /// The source files of the program
    files: BoxcarSimpleFiles<String, String>,
    /// Mapping from indexes to position data
    indices: boxcar::Vec<PosData>,
}

impl Default for PositionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionTable {
    /// The unknown position
    pub const UNKNOWN: PosIdx = PosIdx(0);

    /// Create a new position table where the first file and first position are unknown
    pub fn new() -> Self {
        let table = PositionTable {
            files: BoxcarSimpleFiles::new(),
            indices: boxcar::Vec::new(),
        };
        table.add_file("unknown".to_string(), "".to_string());
        let pos = table.add_pos(FileIdx(0), 0, 0);
        debug_assert!(pos == Self::UNKNOWN);
        table
    }

    /// Return handle to the files in the position table
    pub fn files(&self) -> &BoxcarSimpleFiles<String, String> {
        &self.files
    }

    /// Add a new file to the position table
    pub fn add_file(&self, name: String, source: String) -> FileIdx {
        let idx = self.files.add(name, source);
        FileIdx(idx)
    }

    pub fn get_file_info(&self, pos: PosIdx) -> (&str, &str) {
        let pos_d = self.get_pos(pos);
        self.get_file_data(pos_d.file)
    }

    /// Add a new position to the position table
    pub fn add_pos(&self, file: FileIdx, start: usize, end: usize) -> PosIdx {
        let pos = PosData { file, start, end };
        let pos_idx = self.indices.count();
        self.indices.push(pos);
        PosIdx(pos_idx as u32)
    }

    pub fn get_pos(&self, pos: PosIdx) -> &PosData {
        &self.indices[pos.0 as usize]
    }

    /// Return the name and source of the file
    pub fn get_file_data(&self, file: FileIdx) -> (&str, &str) {
        let file = &self.files.get(file.0).unwrap();
        (file.name(), file.source())
    }
}

/// The global position table
pub struct GlobalPositionTable;

impl GlobalPositionTable {
    /// Return a reference to the global position table
    pub fn get() -> &'static PositionTable {
        static SINGLETON: sync::LazyLock<PositionTable> =
            sync::LazyLock::new(PositionTable::new);

        &SINGLETON
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
/// A position index backed by a global [PositionTable]
pub struct GPosIdx(pub PosIdx);

impl GPosIdx {
    /// Symbol for the unknown position.
    /// All [PositionTable] must allocate the first index for this use.
    pub const UNKNOWN: GPosIdx = GPosIdx(PosIdx(0));

    /// Convert the position into an optional.
    /// Returns `None` if the position is the unknown position.
    pub fn into_option(self) -> Option<Self> {
        if self == Self::UNKNOWN {
            None
        } else {
            Some(self)
        }
    }

    /// Convert this into a Primary label
    pub fn primary(self) -> Label<usize> {
        assert!(
            self != Self::UNKNOWN,
            "unknown position cannot be converted into label"
        );
        let table = GlobalPositionTable::get();
        let pos = table.get_pos(self.0);
        Label::primary(pos.file.get(), pos.start..pos.end)
    }

    /// Convert this into a Secondary label
    pub fn secondary(self) -> Label<usize> {
        assert!(
            self != Self::UNKNOWN,
            "unknown position cannot be converted into label"
        );
        let table = GlobalPositionTable::get();
        let pos = table.get_pos(self.0);
        Label::secondary(pos.file.get(), pos.start..pos.end)
    }
}
