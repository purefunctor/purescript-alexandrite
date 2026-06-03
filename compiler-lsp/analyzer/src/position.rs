use async_lsp::lsp_types;
use line_index::{LineCol, LineIndex, WideEncoding, WideLineCol};
use rowan::ast::AstNode;
use rowan::{TextRange, TextSize};
use syntax::{SyntaxNode, SyntaxNodePtr, cst};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionEncoding {
    Utf8,
    Utf16,
    Utf32,
}

impl PositionEncoding {
    fn wide(self) -> Option<WideEncoding> {
        WideEncoding::try_from(self).ok()
    }
}

impl TryFrom<PositionEncoding> for WideEncoding {
    type Error = ();

    fn try_from(encoding: PositionEncoding) -> Result<WideEncoding, ()> {
        match encoding {
            PositionEncoding::Utf8 => Err(()),
            PositionEncoding::Utf16 => Ok(WideEncoding::Utf16),
            PositionEncoding::Utf32 => Ok(WideEncoding::Utf32),
        }
    }
}

impl From<PositionEncoding> for lsp_types::PositionEncodingKind {
    fn from(encoding: PositionEncoding) -> lsp_types::PositionEncodingKind {
        match encoding {
            PositionEncoding::Utf8 => lsp_types::PositionEncodingKind::UTF8,
            PositionEncoding::Utf16 => lsp_types::PositionEncodingKind::UTF16,
            PositionEncoding::Utf32 => lsp_types::PositionEncodingKind::UTF32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utf8Position {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utf8Range {
    pub start: Utf8Position,
    pub end: Utf8Position,
}

pub fn protocol_position_to_utf8(
    content: &str,
    position: lsp_types::Position,
    encoding: PositionEncoding,
) -> Option<Utf8Position> {
    let line_index = LineIndex::new(content);
    let line_col = match encoding.wide() {
        None => LineCol { line: position.line, col: position.character },
        Some(encoding) => line_index
            .to_utf8(encoding, WideLineCol { line: position.line, col: position.character })?,
    };

    let position = Utf8Position { line: line_col.line, column: line_col.col };
    let offset = utf8_position_to_offset(content, position)?;
    offset_to_utf8_position(content, offset)
}

pub fn utf8_position_to_protocol(
    content: &str,
    position: Utf8Position,
    encoding: PositionEncoding,
) -> Option<lsp_types::Position> {
    let line_index = LineIndex::new(content);
    let line_col = LineCol { line: position.line, col: position.column };

    let offset = line_index.offset(line_col)?;
    line_index.try_line_col(offset)?;

    let position = match encoding.wide() {
        None => lsp_types::Position { line: line_col.line, character: line_col.col },
        Some(encoding) => {
            let line_col = line_index.to_wide(encoding, line_col)?;
            lsp_types::Position { line: line_col.line, character: line_col.col }
        }
    };

    Some(position)
}

pub fn utf8_range_to_protocol(
    content: &str,
    range: Utf8Range,
    encoding: PositionEncoding,
) -> Option<lsp_types::Range> {
    let start = utf8_position_to_protocol(content, range.start, encoding)?;
    let end = utf8_position_to_protocol(content, range.end, encoding)?;
    Some(lsp_types::Range { start, end })
}

pub fn utf8_position_to_offset(content: &str, position: Utf8Position) -> Option<TextSize> {
    let line_index = LineIndex::new(content);

    let line_range = line_index.line(position.line)?;
    let line_content = content[line_range].trim_end_matches(['\n', '\r']);

    let column = if line_content.is_empty() {
        0
    } else if position.column > line_content.len() as u32 {
        line_content.len() as u32
    } else {
        line_content.get(position.column as usize..)?;
        position.column
    };

    let line_col = LineCol { line: position.line, col: column };
    let offset = line_index.offset(line_col)?;
    line_index.try_line_col(offset)?;
    Some(offset)
}

pub fn offset_to_utf8_position(content: &str, offset: TextSize) -> Option<Utf8Position> {
    let line_index = LineIndex::new(content);
    let LineCol { line, col } = line_index.try_line_col(offset)?;
    Some(Utf8Position { line, column: col })
}

pub fn text_range_to_utf8_range(content: &str, range: TextRange) -> Option<Utf8Range> {
    let start = offset_to_utf8_position(content, range.start())?;
    let end = offset_to_utf8_position(content, range.end())?;
    Some(Utf8Range { start, end })
}

pub fn text_range_to_protocol(
    content: &str,
    range: TextRange,
    encoding: PositionEncoding,
) -> Option<lsp_types::Range> {
    let range = text_range_to_utf8_range(content, range)?;
    utf8_range_to_protocol(content, range, encoding)
}

pub fn declaration_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let declaration = cst::Declaration::cast(node.clone())?;

    macro_rules! declaration_name_range {
        ($declaration:expr, $($variant:ident),+ $(,)?) => {
            match $declaration {
                $(cst::Declaration::$variant(declaration) => declaration.name_token()?.text_range(),)+
                _ => return None,
            }
        };
    }

    let range = match declaration {
        cst::Declaration::ClassDeclaration(declaration) => {
            declaration.class_head()?.name_token()?.text_range()
        }
        cst::Declaration::DeriveDeclaration(declaration) => {
            declaration.instance_name()?.name_token()?.text_range()
        }
        cst::Declaration::InstanceChain(_) => return None,
        declaration => declaration_name_range!(
            declaration,
            ValueSignature,
            ValueEquation,
            DataSignature,
            DataEquation,
            NewtypeSignature,
            NewtypeEquation,
            TypeSynonymSignature,
            TypeSynonymEquation,
            ClassSignature,
            TypeRoleDeclaration,
            ForeignImportDataDeclaration,
            ForeignImportValueDeclaration,
        ),
    };

    text_range_to_utf8_range(content, range)
}

pub fn data_constructor_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let constructor = cst::DataConstructor::cast(node)?;
    let token = constructor.name_token()?;
    text_range_to_utf8_range(content, token.text_range())
}

pub fn class_member_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let member = cst::ClassMemberStatement::cast(node)?;
    let token = member.name_token()?;
    text_range_to_utf8_range(content, token.text_range())
}

pub fn instance_declaration_name_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let instance = cst::InstanceDeclaration::cast(node)?;
    let token = instance.instance_name()?.name_token()?;
    text_range_to_utf8_range(content, token.text_range())
}

pub fn infix_operator_range(
    content: &str,
    root: &SyntaxNode,
    ptr: &SyntaxNodePtr,
) -> Option<Utf8Range> {
    let node = ptr.try_to_node(root)?;
    let declaration = cst::InfixDeclaration::cast(node)?;
    let token = declaration.operator_token()?;
    text_range_to_utf8_range(content, token.text_range())
}

#[cfg(test)]
mod tests {
    use async_lsp::lsp_types::Position;
    use rowan::TextSize;

    use super::{
        PositionEncoding, Utf8Position, offset_to_utf8_position, protocol_position_to_utf8,
        utf8_position_to_offset, utf8_position_to_protocol,
    };

    #[test]
    fn utf16_protocol_position_maps_to_utf8_column() {
        let content = "a😀b";
        let position = Position::new(0, 3);

        let position =
            protocol_position_to_utf8(content, position, PositionEncoding::Utf16).unwrap();
        assert_eq!(position, Utf8Position { line: 0, column: 5 });

        let offset = utf8_position_to_offset(content, position);
        assert_eq!(offset, Some(TextSize::new(5)));
    }

    #[test]
    fn utf32_protocol_position_maps_to_utf8_column() {
        let content = "a😀b";
        let position = Position::new(0, 2);

        let position =
            protocol_position_to_utf8(content, position, PositionEncoding::Utf32).unwrap();
        assert_eq!(position, Utf8Position { line: 0, column: 5 });
    }

    #[test]
    fn utf8_position_maps_to_utf16_protocol_position() {
        let content = "a😀b";
        let position = offset_to_utf8_position(content, TextSize::new(5)).unwrap();

        let position =
            utf8_position_to_protocol(content, position, PositionEncoding::Utf16).unwrap();
        assert_eq!(position, Position::new(0, 3));
    }

    #[test]
    fn protocol_position_past_line_end_clamps_to_same_line() {
        let content = "abc\ndef";
        let position = Position::new(0, 99);

        let position =
            protocol_position_to_utf8(content, position, PositionEncoding::Utf16).unwrap();
        assert_eq!(position, Utf8Position { line: 0, column: 3 });
    }
}
