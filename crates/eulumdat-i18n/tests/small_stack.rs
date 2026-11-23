//! Windows gives the main thread 1 MB of stack (Linux and macOS 8 MB). Loading and
//! holding a few locales must fit into that, in a debug build too.
use eulumdat_i18n::{Language, Locale};

#[test]
fn locales_load_on_a_one_megabyte_stack() {
    eprintln!(
        "size_of::<Locale>() = {} bytes",
        std::mem::size_of::<Locale>()
    );
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let en = Locale::english();
            assert_eq!(en.diagram.axis.gamma, "Gamma (γ)");
            let de = Locale::german();
            assert_eq!(de.diagram.axis.intensity, "Lichtstärke (cd/klm)");
            let zh = Locale::for_language(Language::Chinese);
            assert_eq!(zh.diagram.placeholder.no_data, "无数据");
        })
        .unwrap()
        .join()
        .expect("locales fit a 1 MB stack");
}
