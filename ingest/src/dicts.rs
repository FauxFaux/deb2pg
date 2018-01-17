const DICT_LEN: usize = 112640;
type Dict = &'static [u8; DICT_LEN];

const DICT_11_TEXT: Dict = include_bytes!("../../dicts/text.dictionary");
const DICT_12_CONF: Dict = include_bytes!("../../dicts/conf.dictionary");
const DICT_13_C: Dict = include_bytes!("../../dicts/c.dictionary");
const DICT_14_CODE: Dict = include_bytes!("../../dicts/oddlang.dictionary");
const DICT_15_WEB: Dict = include_bytes!("../../dicts/web.dictionary");
const DICT_2_99: Dict = include_bytes!("../../dicts/99.dictionary");
const DICT_3_999: Dict = include_bytes!("../../dicts/999.dictionary");
const DICT_4_9999: Dict = include_bytes!("../../dicts/9999.dictionary");

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CompressionType {
    Text,
    Conf,
    C,
    Code,
    Web,

    Tiny,
    Medium,
    Other,
}

pub fn dict_for(format: CompressionType) -> Dict {
    match format {
        CompressionType::Text => DICT_11_TEXT,
        CompressionType::Conf => DICT_12_CONF,
        CompressionType::C => DICT_13_C,
        CompressionType::Code => DICT_14_CODE,
        CompressionType::Web => DICT_15_WEB,
        CompressionType::Tiny => DICT_2_99,
        CompressionType::Medium => DICT_3_999,
        CompressionType::Other => DICT_4_9999,
    }
}

pub fn identify(path: &str) -> CompressionType {
    let filename: String = path.split('/').last().unwrap_or(path).to_ascii_lowercase();
    let ext = &filename[filename.rfind('.').unwrap_or(0)..];
    use self::CompressionType::*;

    match ext {
        "txt" => Text,
        "rst" => Text,
        "md" => Text,
        "doc" => Text,

        "control" => Text,
        "dsc" => Text,
        "readme" => Text,
        "license" => Text,
        "licence" => Text,
        "copyright" => Text,

        "po" => Text,
        "mo" => Text,

        "rdf" => Text,
        "qdoc" => Text,
        "manifest" => Text,

        "makefile" => Conf,
        "in" => Conf,
        "m4" => Conf,
        "mk" => Conf,
        "pro" => Conf,
        "rules" => Conf,
        "inc" => Conf,
        "am" => Conf,
        "build" => Conf,
        "cmake" => Conf,
        "gyp" => Conf,

        "gitignore" => Conf,

        "properties" => Conf,
        "ini" => Conf,
        "json" => Conf,
        "cfg" => Conf,
        "conf" => Conf,
        "yaml" => Conf,
        "qml" => Conf,

        "sh" => Conf,

        "c" => C,
        "cpp" => C,
        "cc" => C,
        "cxx" => C,
        "h" => C,
        "hpp" => C,
        "hxx" => C,

        "d" => C,
        "ml" => C,
        "hh" => C,
        "idl" => C,

        "cs" => Code,
        "go" => Code,
        "hs" => Code,
        "java" => Code,
        "ll" => Code,
        "php" => Code,
        "phpt" => Code,
        "pl" => Code,
        "pm" => Code,
        "py" => Code,
        "rb" => Code,
        "rs" => Code,
        "s" => Code,

        "htm" => Web,
        "html" => Web,
        "xhtml" => Web,

        "js" => Web,
        "sjs" => Web,

        "dtd" => Web,
        "xml" => Web,
        "svg" => Web,

        "xht" => Web,
        "xul" => Web,

        "css" => Web,

        _ => {
            if filename.contains("readme") || filename.contains("license")
                || filename.contains("licence")
            {
                Text
            } else {
                Other
            }
        }
    }
}
