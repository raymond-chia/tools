// 技能編輯器功能
class SkillEditor {
  constructor() {
    this.selectedFile = null;
    this.skillsData = null;
    this.selectedSkill = null;
    this.effectIdCounter = 0;

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
    this.saveBtn = document.getElementById("save-btn");
    this.deleteBtn = document.getElementById("delete-skill-btn");

    // 標籤選擇器
    this.tagCheckboxes = document.querySelectorAll(".tag-checkbox");
    this.tagRadios = document.querySelectorAll(".tag-radio");

    // 基本屬性
    this.skillRange = document.getElementById("skill-range");
    this.skillArea = document.getElementById("skill-area");
    this.skillCost = document.getElementById("skill-cost");
    this.skillHitRate = document.getElementById("skill-hit-rate");
    this.skillCritRate = document.getElementById("skill-crit-rate");

    // 效果相關
    this.effectsContainer = document.getElementById("effects-container");
    this.addHpEffectBtn = document.getElementById("add-hp-effect");
    this.addBurnEffectBtn = document.getElementById("add-burn-effect");
  }

  // 初始化事件監聽器
  initEventListeners() {
    this.selectFileBtn.addEventListener("click", () => this.handleFileSelect());
    this.saveBtn.addEventListener("click", () => this.handleSaveSkill());
    document
      .getElementById("new-skill-btn")
      .addEventListener("click", () => this.handleNewSkill());
    this.deleteBtn.addEventListener("click", () => this.handleDeleteSkill());

    // 效果按鈕
    this.addHpEffectBtn.addEventListener("click", () => this.addEffect("hp"));
    this.addBurnEffectBtn.addEventListener("click", () =>
      this.addEffect("burn")
    );
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

      // 獲取技能類型標籤
      const hasActiveTag =
        skill.tags &&
        skill.tags.some((tag) => tag === "active" || tag === "Active");
      const hasBeneficialTag =
        skill.tags &&
        skill.tags.some((tag) => tag === "beneficial" || tag === "Beneficial");

      // 創建內容
      itemElement.innerHTML = `
        <div class="skill-name">${skillKey}</div>
        <div class="skill-key">${!hasActiveTag ? "（被動）" : "（主動）"} ${
        hasBeneficialTag ? "（有益）" : "（有害）"
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

    // 清除效果容器
    this.effectsContainer.innerHTML = "";
    this.effectIdCounter = 0;

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

      // 重置所有標籤
      this.tagCheckboxes.forEach((checkbox) => {
        checkbox.checked = false;
      });

      this.tagRadios.forEach((radio) => {
        radio.checked = false;
      });

      // 設置標籤
      if (skill.tags && Array.isArray(skill.tags)) {
        skill.tags.forEach((tag) => {
          const tagName =
            typeof tag === "string" ? tag.toLowerCase() : tag.toLowerCase();

          // 檢查複選框
          const checkbox = document.querySelector(
            `.tag-checkbox[data-tag="${tagName}"]`
          );
          if (checkbox) {
            checkbox.checked = true;
          }

          // 檢查單選按鈕
          const radio = document.querySelector(
            `.tag-radio[data-tag="${tagName}"]`
          );
          if (radio) {
            radio.checked = true;
          }
        });
      }

      // 設置基本屬性
      this.skillRange.value = skill.range || 0;
      this.skillArea.value = skill.area || 0;
      this.skillCost.value = skill.cost || 0;
      this.skillHitRate.value = skill.hit_rate || "";
      this.skillCritRate.value = skill.crit_rate || "";

      // 添加效果
      if (skill.effects && Array.isArray(skill.effects)) {
        skill.effects.forEach((effect) => {
          if (effect.type === "hp") {
            this.addEffect("hp", effect.target_type, effect.value);
          } else if (effect.type === "burn") {
            this.addEffect("burn", null, null, effect.duration);
          }
        });
      }

      // 顯示詳情面板
      this.skillDetail.classList.remove("hidden");
    } else {
      // 隱藏詳情面板
      this.skillDetail.classList.add("hidden");
    }
  }

  // 添加效果
  addEffect(type, targetType = null, value = null, duration = null) {
    const effectId = `effect-${this.effectIdCounter++}`;
    const effectElement = document.createElement("div");
    effectElement.className = "effect-item";
    effectElement.dataset.effectId = effectId;
    effectElement.dataset.effectType = type;

    const effectHeader = document.createElement("div");
    effectHeader.className = "effect-header";

    const effectTitle = document.createElement("div");
    effectTitle.className = "effect-title";
    effectTitle.textContent = type === "hp" ? "HP 效果" : "燃燒效果";

    const removeButton = document.createElement("button");
    removeButton.className = "remove-effect";
    removeButton.textContent = "×";
    removeButton.addEventListener("click", () => {
      effectElement.remove();
    });

    effectHeader.appendChild(effectTitle);
    effectHeader.appendChild(removeButton);
    effectElement.appendChild(effectHeader);

    const effectFields = document.createElement("div");
    effectFields.className = "effect-fields";

    // 根據效果類型添加不同的欄位
    if (type === "hp") {
      // 目標類型選擇器
      const targetTypeField = document.createElement("div");
      targetTypeField.className = "field";

      const targetTypeLabel = document.createElement("label");
      targetTypeLabel.textContent = "目標類型：";
      targetTypeLabel.htmlFor = `${effectId}-target-type`;

      const targetTypeSelect = document.createElement("select");
      targetTypeSelect.id = `${effectId}-target-type`;
      targetTypeSelect.name = "target_type";

      const targetOptions = [
        { value: "caster", text: "施法者" },
        { value: "ally", text: "友方" },
        { value: "ally_exclude_caster", text: "友方（不包括施法者）" },
        { value: "enemy", text: "敵人" },
        { value: "any", text: "任何單位" },
        { value: "any_exclude_caster", text: "任何單位（不包括施法者）" },
      ];

      targetOptions.forEach((option) => {
        const optionElement = document.createElement("option");
        optionElement.value = option.value;
        optionElement.textContent = option.text;

        // 如果有傳入的目標類型，設為選中
        if (targetType && option.value === targetType.toLowerCase()) {
          optionElement.selected = true;
        }

        targetTypeSelect.appendChild(optionElement);
      });

      targetTypeField.appendChild(targetTypeLabel);
      targetTypeField.appendChild(targetTypeSelect);
      effectFields.appendChild(targetTypeField);

      // 數值輸入
      const valueField = document.createElement("div");
      valueField.className = "field";

      const valueLabel = document.createElement("label");
      valueLabel.textContent = "數值 (負數為傷害)：";
      valueLabel.htmlFor = `${effectId}-value`;

      const valueInput = document.createElement("input");
      valueInput.type = "number";
      valueInput.id = `${effectId}-value`;
      valueInput.name = "value";
      valueInput.value = value !== null ? value : 0;

      valueField.appendChild(valueLabel);
      valueField.appendChild(valueInput);
      effectFields.appendChild(valueField);
    } else if (type === "burn") {
      // 持續時間
      const durationField = document.createElement("div");
      durationField.className = "field";

      const durationLabel = document.createElement("label");
      durationLabel.textContent = "持續回合：";
      durationLabel.htmlFor = `${effectId}-duration`;

      const durationInput = document.createElement("input");
      durationInput.type = "number";
      durationInput.id = `${effectId}-duration`;
      durationInput.name = "duration";
      durationInput.min = "1";
      durationInput.value = duration !== null ? duration : 1;

      durationField.appendChild(durationLabel);
      durationField.appendChild(durationInput);
      effectFields.appendChild(durationField);
    }

    effectElement.appendChild(effectFields);
    this.effectsContainer.appendChild(effectElement);
  }

  // 從表單中收集效果數據
  collectEffects() {
    const effects = [];
    const effectElements =
      this.effectsContainer.querySelectorAll(".effect-item");

    effectElements.forEach((element) => {
      const effectType = element.dataset.effectType;

      if (effectType === "hp") {
        const targetType = element.querySelector(
          "select[name='target_type']"
        ).value;
        const value = parseInt(
          element.querySelector("input[name='value']").value || 0
        );

        effects.push({
          type: "hp",
          target_type: targetType,
          value: value,
        });
      } else if (effectType === "burn") {
        const duration = parseInt(
          element.querySelector("input[name='duration']").value || 1
        );

        effects.push({
          type: "burn",
          duration: duration,
        });
      }
    });

    return effects;
  }

  // 保存技能
  async handleSaveSkill() {
    if (!this.selectedSkill) return;

    // 收集標籤
    const selectedTags = [];

    // 收集複選框標籤
    this.tagCheckboxes.forEach((checkbox) => {
      if (checkbox.checked) {
        selectedTags.push(checkbox.dataset.tag);
      }
    });

    // 收集單選按鈕標籤
    this.tagRadios.forEach((radio) => {
      if (radio.checked) {
        selectedTags.push(radio.dataset.tag);
      }
    });

    // 收集基本屬性
    const range = parseInt(this.skillRange.value || 0);
    const area = parseInt(this.skillArea.value || 0);
    const cost = parseInt(this.skillCost.value || 0);

    // 命中率和暴擊率可能為空
    const hitRateStr = this.skillHitRate.value.trim();
    const critRateStr = this.skillCritRate.value.trim();

    const hitRate = hitRateStr ? parseInt(hitRateStr) : null;
    const critRate = critRateStr ? parseInt(critRateStr) : null;

    // 收集效果
    const effects = this.collectEffects();

    // 創建技能數據對象
    const skillData = {
      tags: selectedTags,
      range,
      area,
      cost,
      hit_rate: hitRate,
      crit_rate: critRate,
      effects,
    };

    try {
      await api.saveSkill(this.selectedFile, this.selectedSkill, skillData);

      // 更新本地資料，避免重新載入
      if (this.skillsData && this.skillsData.skills) {
        // 直接替換整個技能對象，確保 effects 數組被完整保留
        this.skillsData.skills[this.selectedSkill] = skillData;

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
