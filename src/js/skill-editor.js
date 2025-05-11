// 技能編輯器功能
class SkillEditor {
  constructor() {
    this.selectedFile = null;
    this.skillsData = null;
    this.selectedSkill = null;

    // 初始化 UI 元素引用
    this.initElements();

    // 初始化事件監聽器
    this.initEventListeners();

    // 初始化確認對話框
    this.confirmModalPromise = null;
    this.confirmDialog = document.getElementById("confirm-dialog");
    this.confirmMessage = document.getElementById("confirm-message");
    this.confirmOkBtn = document.getElementById("confirm-ok-btn");
    this.confirmCancelBtn = document.getElementById("confirm-cancel-btn");
    this.isConfirmProcessing = false;

    this.initConfirmDialog();
  }

  // 初始化確認對話框
  initConfirmDialog() {
    // 點擊確定按鈕
    this.confirmOkBtn.addEventListener("click", () => {
      if (this.confirmModalPromise) {
        this.confirmModalPromise.resolve(true);
        this.confirmModalPromise = null;
        this.isConfirmProcessing = false;
      }
      this.confirmDialog.classList.add("hidden");
    });

    // 點擊取消按鈕或按 ESC
    const handleCancel = () => {
      if (this.confirmModalPromise) {
        this.confirmModalPromise.resolve(false);
        this.confirmModalPromise = null;
        this.isConfirmProcessing = false;
      }
      this.confirmDialog.classList.add("hidden");
    };

    this.confirmCancelBtn.addEventListener("click", handleCancel);

    // 點擊背景關閉
    this.confirmDialog.addEventListener("click", (e) => {
      if (e.target === this.confirmDialog) {
        handleCancel();
      }
    });

    // ESC 鍵關閉
    document.addEventListener("keydown", (e) => {
      if (
        e.key === "Escape" &&
        !this.confirmDialog.classList.contains("hidden")
      ) {
        handleCancel();
      }
    });
  }

  // 顯示確認對話框
  showConfirmDialog(message) {
    // 如果正在處理中，直接返回 false
    if (this.isConfirmProcessing) {
      return Promise.resolve(false);
    }

    return new Promise((resolve) => {
      this.isConfirmProcessing = true;

      // 如果已經有未完成的對話框，先取消它
      if (this.confirmModalPromise) {
        this.confirmModalPromise.resolve(false);
        this.confirmDialog.classList.add("hidden");
      }

      // 更新對話框內容
      this.confirmMessage.textContent = message;
      this.confirmDialog.classList.remove("hidden");

      // 保存新的 promise 和清理函數
      this.confirmModalPromise = {
        resolve: (result) => {
          this.isConfirmProcessing = false;
          resolve(result);
        },
      };
    });
  }

  // 初始化 UI 元素引用
  initElements() {
    // 檔案選擇相關
    this.selectFileBtn = document.getElementById("select-file-btn");
    this.fileInfo = document.getElementById("file-info");
    this.currentFile = document.getElementById("current-file");

    // 內容區域
    this.editorContent = document.getElementById("editor-content");
    this.emptyState = document.getElementById("empty-state");

    // 技能列表
    this.skillItems = document.getElementById("skill-items");
    this.skillCount = document.getElementById("skill-count");

    // 技能詳情
    this.skillDetail = document.getElementById("skill-detail");
    this.skillIdElement = document.getElementById("skill-id");
    this.skillActive = document.getElementById("skill-active");
    this.skillBeneficial = document.getElementById("skill-beneficial");
    this.saveBtn = document.getElementById("save-btn");
    this.deleteBtn = document.getElementById("delete-skill-btn");
  }

  // 初始化事件監聽器
  initEventListeners() {
    this.selectFileBtn.addEventListener("click", () => this.handleFileSelect());
    this.saveBtn.addEventListener("click", () => this.handleSaveSkill());
    document
      .getElementById("new-skill-btn")
      .addEventListener("click", () => this.handleNewSkill());
    this.deleteBtn.addEventListener("click", () => this.handleDeleteSkill());
  }

  // 處理刪除技能
  async handleDeleteSkill() {
    if (!this.selectedSkill) return;

    // 第一次確認
    const firstConfirmed = await this.showConfirmDialog(
      `確定要刪除技能 "${this.selectedSkill}" 嗎？\n此操作無法復原。`
    );
    if (!firstConfirmed) return;

    // 第二次確認
    const secondConfirmed = await this.showConfirmDialog(
      "再次確認：你真的要刪除這個技能嗎？"
    );
    if (!secondConfirmed) return;

    // 兩次都確認後才執行刪除
    await this.executeDelete();
  }

  // 執行刪除操作
  async executeDelete() {
    try {
      await api.deleteSkill(this.selectedFile, this.selectedSkill);

      // 重新載入技能列表
      await this.loadSkills(this.selectedFile);

      // 隱藏詳情面板
      this.selectSkill(null);

      alert("刪除技能成功!");
    } catch (error) {
      alert(`刪除技能失敗: ${error}`);
    }
  }

  // 處理新增技能
  async handleNewSkill() {
    if (!this.selectedFile) {
      alert("請先選擇技能檔案");
      return;
    }

    const skillId = prompt("請輸入技能 ID (英文、數字或底線):");
    if (!skillId) return;

    if (!/^[a-zA-Z0-9_]+$/.test(skillId)) {
      alert("技能 ID 只能包含英文、數字或底線");
      return;
    }

    try {
      await api.createSkill(this.selectedFile, skillId);

      // 重新載入技能列表
      await this.loadSkills(this.selectedFile);

      // 選中新建的技能
      this.selectSkill(skillId);

      alert("新增技能成功!");
    } catch (error) {
      alert(`新增技能失敗: ${error}`);
    }
  }

  // 處理檔案選擇
  async handleFileSelect() {
    try {
      const selectedPath = await api.selectFile();

      if (!selectedPath) {
        return; // 使用者取消了選擇
      }

      // 驗證檔案
      try {
        const filePath = await api.checkFile(selectedPath);
        this.selectedFile = filePath;
        this.currentFile.textContent = `當前檔案: ${filePath}`;

        // 顯示檔案資訊
        this.fileInfo.classList.remove("hidden");

        // 載入技能資料
        await this.loadSkills(filePath);
      } catch (error) {
        alert(error);
      }
    } catch (error) {
      console.error("選擇檔案失敗:", error);
    }
  }

  // 載入技能資料
  async loadSkills(filePath) {
    try {
      const data = await api.loadSkills(filePath);
      this.skillsData = data;

      // 更新界面
      this.updateSkillList();

      // 顯示編輯器內容
      this.emptyState.classList.add("hidden");
      this.editorContent.classList.remove("hidden");

      // 重置選擇的技能
      this.selectSkill(null);
    } catch (error) {
      alert(`載入失敗: ${error}`);
    }
  }

  // 更新技能列表
  updateSkillList() {
    // 清空現有列表
    this.skillItems.innerHTML = "";

    // 獲取排序後的技能列表
    const skillEntries = Object.entries(this.skillsData.skills).sort(
      ([keyA], [keyB]) => keyA.localeCompare(keyB)
    );

    // 更新技能數量
    this.skillCount.textContent = `${skillEntries.length} 個技能`;

    // 添加技能項目
    skillEntries.forEach(([skillKey, skill]) => {
      const itemElement = document.createElement("div");
      itemElement.className = "skill-item";
      itemElement.dataset.skillId = skillKey;

      // 如果是選中的技能，添加選中樣式
      if (this.selectedSkill === skillKey) {
        itemElement.classList.add("selected");
      }

      // 添加點擊事件
      itemElement.addEventListener("click", () => this.selectSkill(skillKey));

      // 創建內容
      itemElement.innerHTML = `
        <div class="skill-name">${skillKey}</div>
        <div class="skill-key">${!skill.is_active ? "（被動）" : "（主動）"} ${
        skill.is_beneficial ? "（有益）" : "（有害）"
      }</div>
      `;

      this.skillItems.appendChild(itemElement);
    });

    // 如果沒有技能，顯示提示
    if (skillEntries.length === 0) {
      const noSkillsMsg = document.createElement("div");
      noSkillsMsg.className = "no-skills-message";
      noSkillsMsg.textContent = "沒有找到技能";
      this.skillItems.appendChild(noSkillsMsg);
    }
  }

  // 選擇技能
  selectSkill(skillId) {
    this.selectedSkill = skillId;

    // 更新技能列表選中狀態
    const skillItems = this.skillItems.querySelectorAll(".skill-item");
    skillItems.forEach((item) => {
      if (item.dataset.skillId === skillId) {
        item.classList.add("selected");
      } else {
        item.classList.remove("selected");
      }
    });

    // 如果選了技能，顯示詳情
    if (skillId && this.skillsData.skills[skillId]) {
      const skill = this.skillsData.skills[skillId];

      // 更新技能詳情
      this.skillIdElement.textContent = `ID: ${skillId}`;
      this.skillActive.checked = skill.is_active || true;
      this.skillBeneficial.checked = skill.is_beneficial || false;

      // 顯示詳情面板
      this.skillDetail.classList.remove("hidden");
    } else {
      // 隱藏詳情面板
      this.skillDetail.classList.add("hidden");
    }
  }

  // 保存技能
  async handleSaveSkill() {
    if (!this.selectedSkill) return;

    const isActive = this.skillActive.checked;
    const isBeneficial = this.skillBeneficial.checked;

    try {
      await api.saveSkill(
        this.selectedFile,
        this.selectedSkill,
        isActive,
        isBeneficial
      );

      // 更新本地資料，避免重新載入
      if (this.skillsData && this.skillsData.skills) {
        if (!this.skillsData.skills[this.selectedSkill]) {
          this.skillsData.skills[this.selectedSkill] = {};
        }

        this.skillsData.skills[this.selectedSkill].is_active = isActive;
        this.skillsData.skills[this.selectedSkill].is_beneficial = isBeneficial;

        // 只更新技能列表，不重新載入整個資料
        this.updateSkillList();
      } else {
        // 如果出現問題，仍然可以回退到完全重新載入
        await this.loadSkills(this.selectedFile);
      }

      // 確保選中的技能仍然選中
      this.selectSkill(this.selectedSkill);

      // 顯示成功訊息
      alert("保存成功!");
    } catch (error) {
      alert(`保存失敗: ${error}`);
    }
  }
}

// 在 DOM 載入完成後初始化技能編輯器
let skillEditor;
document.addEventListener("DOMContentLoaded", () => {
  skillEditor = new SkillEditor();
});
