use building::QueryEngine;
use files::Files;

use crate::position::PositionEncoding;

pub struct LanguageContext<'a> {
    pub engine: &'a QueryEngine,
    pub files: &'a Files,
    pub encoding: PositionEncoding,
}

impl<'a> LanguageContext<'a> {
    pub fn new(
        engine: &'a QueryEngine,
        files: &'a Files,
        encoding: PositionEncoding,
    ) -> LanguageContext<'a> {
        LanguageContext { engine, files, encoding }
    }
}
