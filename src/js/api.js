// Tauri API 封裝
const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

const api = {
  // 檢查檔案類型是否正確
  checkFile: async (path) => {
    try {
      return await invoke("check_file", { path });
    } catch (error) {
      console.error("檢查檔案失敗:", error);
      throw error;
    }
  },

  // 載入技能資料
  loadSkills: async (path) => {
    try {
      return await invoke("load_skills", {
        path,
      });
    } catch (error) {
      console.error("載入技能資料失敗:", error);
      throw error;
    }
  },

  // 保存技能屬性
  saveSkill: async (path, skillId, isActive, isBeneficial) => {
    try {
      return await invoke("save_skill", {
        path,
        skillId,
        isActive,
        isBeneficial,
      });
    } catch (error) {
      console.error("保存技能被動屬性失敗:", error);
      throw error;
    }
  },

  // 新增技能
  createSkill: async (path, skillId) => {
    try {
      return await invoke("create_skill", {
        path,
        skillId,
      });
    } catch (error) {
      console.error("新增技能失敗:", error);
      throw error;
    }
  },

  // 刪除技能
  deleteSkill: async (path, skillId) => {
    try {
      return await invoke("delete_skill", {
        path,
        skillId,
      });
    } catch (error) {
      console.error("刪除技能失敗:", error);
      throw error;
    }
  },

  // 選擇檔案 (使用 Tauri 對話框)
  selectFile: async () => {
    try {
      return await open({
        directory: false,
        multiple: false,
        title: "選擇技能檔案",
        filters: [
          {
            name: "TOML 檔案",
            extensions: ["toml"],
          },
        ],
      });
    } catch (error) {
      console.error("選擇檔案失敗:", error);
      throw error;
    }
  },
};
