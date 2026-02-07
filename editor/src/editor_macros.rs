#[macro_export]
macro_rules! define_editors {
    (
        default: $default_variant:ident,
        $(
            $variant:ident => {
                display: $display:expr,
                field: $field:ident,
                type: $type:ty,
                file_fn: $file_fn:expr,
            }
        ),* $(,)?
    ) => {
        /// 編輯器標籤頁
        #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Display)]
        pub enum EditorTab {
            $(
                #[strum(to_string = $display)]
                $variant,
            )*
        }

        impl Default for EditorTab {
            fn default() -> Self {
                Self::$default_variant
            }
        }

        /// 編輯器應用程式狀態
        #[derive(Debug)]
        pub struct EditorApp {
            pub current_tab: EditorTab,
            $(
                pub $field: GenericEditorState<$type>,
            )*
        }

        impl EditorApp {
            /// 建立編輯器並載入所有資料檔案
            pub fn new() -> Self {
                let mut app = Self {
                    current_tab: EditorTab::default(),
                    $(
                        $field: GenericEditorState::default(),
                    )*
                };

                let data_dir = PathBuf::from(DATA_DIRECTORY_PATH);
                $(
                    {
                        let file_name = $file_fn();
                        load_file(
                            &mut app.$field,
                            &data_dir.join(format!("{}.toml", file_name)),
                            file_name,
                        );
                    }
                )*

                app
            }
        }
    };
}
