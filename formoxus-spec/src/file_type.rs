
#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    Extension(String), // ".pdf", stored lowercase
    Mime(String),      // "application/pdf", stored lowercase
    AnyOf(String),     // "image" for image/*
}

impl FileType {
    pub fn parse(file_type: &str) -> Result<FileType> {
        if file_type.starts_with('.') && file_type.len() > 1 {
            Ok(FileType::Extension(file_type.to_lowercase().to_string()))
        } else if 
    }
}

pu