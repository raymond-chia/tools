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
      // 獲取檔案路徑，優先使用 this.selectedFile，如果 skillsData 有 file_path 則使用
      const filePath =
        this.selectedFile || (this.skillsData && this.skillsData.file_path);
      if (!filePath) {
        throw new Error("找不到檔案路徑");
      }

      console.log("刪除技能:", filePath, "技能ID:", this.selectedSkill);
      await api.deleteSkill(filePath, this.selectedSkill);

      // 重新載入技能列表
      await this.loadSkills(filePath);

      // 隱藏詳情面板
      this.selectSkill(null);

      alert("刪除技能成功!");
    } catch (error) {
      console.error("刪除技能時出錯:", error);
      alert(`刪除技能失敗: ${error}`);
    }
  }

  // 處理新增技能
  async handleNewSkill() {
    // 獲取檔案路徑，優先使用 this.selectedFile，如果 skillsData 有 file_path 則使用
    const filePath =
      this.selectedFile || (this.skillsData && this.skillsData.file_path);
    if (!filePath) {
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
      console.log("新增技能:", filePath, "技能ID:", skillId);
      await api.createSkill(filePath, skillId);

      // 重新載入技能列表
      await this.loadSkills(filePath);

      // 選中新建的技能
      this.selectSkill(skillId);

      alert("新增技能成功!");
    } catch (error) {
      console.error("新增技能時出錯:", error);
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
      const response = await api.loadSkills(filePath);
      // 查看控制台中的數據結構
      console.log("從後端載入的技能數據:", response);

      // 確保 response 有 skills 屬性
      if (!response || !response.skills) {
        throw new Error("技能數據格式無效");
      }

      // 將後端回傳的數據格式標準化為前端需要的格式
      this.skillsData = {
        skills: response.skills,
        file_path: response.file_path || filePath,
      };

      console.log("處理後的技能數據:", this.skillsData);

      // 更新界面
      this.updateSkillList();

      // 顯示編輯器內容
      this.emptyState.classList.add("hidden");
      this.editorContent.classList.remove("hidden");

      // 重置選擇的技能
      this.selectSkill(null);
    } catch (error) {
      console.error("載入技能時出錯:", error);
      alert(`載入失敗: ${error}`);
    }
  }

  // 更新技能列表
  updateSkillList() {
    // 清空現有列表
    this.skillItems.innerHTML = "";

    // 檢查 skillsData 是否有效且含有 skills 屬性
    if (!this.skillsData || !this.skillsData.skills) {
      console.error("技能資料無效:", this.skillsData);
      const noSkillsMsg = document.createElement("div");
      noSkillsMsg.className = "no-skills-message";
      noSkillsMsg.textContent = "技能資料格式無效";
      this.skillItems.appendChild(noSkillsMsg);
      this.skillCount.textContent = "0 個技能";
      return;
    }

    // 獲取排序後的技能列表
    const skillEntries = Object.entries(this.skillsData.skills).sort(
      ([keyA], [keyB]) => keyA.localeCompare(keyB)
    );

    // 更新技能數量
    this.skillCount.textContent = `${skillEntries.length} 個技能`;

    console.log("解析後的技能列表:", skillEntries);

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
    console.log("選擇技能:", skillId);
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

    // 檢查 skillsData 是否有效
    if (!this.skillsData || !this.skillsData.skills) {
      console.error("選擇技能時 skillsData 無效:", this.skillsData);
      return;
    }

    // 如果選了技能，顯示詳情
    if (skillId && this.skillsData.skills[skillId]) {
      console.log("選擇的技能數據:", this.skillsData.skills[skillId]);
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
      this.skillCost.value = skill.cost || 0;
      this.skillHitRate.value = skill.hit_rate || "";
      this.skillCritRate.value = skill.crit_rate || "";

      // 添加效果
      if (skill.effects && Array.isArray(skill.effects)) {
        skill.effects.forEach((effect) => {
          if (effect.type === "hp") {
            this.addEffect(
              "hp",
              effect.target_type,
              effect.value,
              null,
              effect.shape
            );
          } else if (effect.type === "burn") {
            this.addEffect(
              "burn",
              effect.target_type,
              null,
              effect.duration,
              effect.shape
            );
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
  addEffect(
    type,
    targetType = null,
    value = null,
    duration = null,
    shape = null
  ) {
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

    // 形狀選擇器
    const shapeField = document.createElement("div");
    shapeField.className = "field";

    const shapeLabel = document.createElement("label");
    shapeLabel.textContent = "效果形狀：";
    shapeLabel.htmlFor = `${effectId}-shape-type`;

    const shapeSelect = document.createElement("select");
    shapeSelect.id = `${effectId}-shape-type`;
    shapeSelect.name = "shape_type";
    shapeSelect.addEventListener("change", (e) => {
      // 顯示或隱藏相應的區域參數
      const shapeParamsContainer = effectElement.querySelector(".shape-params");
      if (shapeParamsContainer) {
        const shapeType = e.target.value;

        // 清空容器
        shapeParamsContainer.innerHTML = "";

        // 根據形狀類型添加不同參數
        if (shapeType === "circle") {
          const areaField = document.createElement("div");
          areaField.className = "field";

          const areaLabel = document.createElement("label");
          areaLabel.textContent = "半徑：";
          areaLabel.htmlFor = `${effectId}-shape-area`;

          const areaInput = document.createElement("input");
          areaInput.type = "number";
          areaInput.id = `${effectId}-shape-area`;
          areaInput.name = "shape_area";
          areaInput.min = "1";
          areaInput.value = "1";

          areaField.appendChild(areaLabel);
          areaField.appendChild(areaInput);
          shapeParamsContainer.appendChild(areaField);
        } else if (shapeType === "rectangle") {
          // 寬度
          const widthField = document.createElement("div");
          widthField.className = "field";

          const widthLabel = document.createElement("label");
          widthLabel.textContent = "寬度：";
          widthLabel.htmlFor = `${effectId}-shape-width`;

          const widthInput = document.createElement("input");
          widthInput.type = "number";
          widthInput.id = `${effectId}-shape-width`;
          widthInput.name = "shape_width";
          widthInput.min = "1";
          widthInput.value = "1";

          widthField.appendChild(widthLabel);
          widthField.appendChild(widthInput);
          shapeParamsContainer.appendChild(widthField);

          // 高度
          const heightField = document.createElement("div");
          heightField.className = "field";

          const heightLabel = document.createElement("label");
          heightLabel.textContent = "高度：";
          heightLabel.htmlFor = `${effectId}-shape-height`;

          const heightInput = document.createElement("input");
          heightInput.type = "number";
          heightInput.id = `${effectId}-shape-height`;
          heightInput.name = "shape_height";
          heightInput.min = "1";
          heightInput.value = "1";

          heightField.appendChild(heightLabel);
          heightField.appendChild(heightInput);
          shapeParamsContainer.appendChild(heightField);
        } else if (shapeType === "line") {
          const lengthField = document.createElement("div");
          lengthField.className = "field";

          const lengthLabel = document.createElement("label");
          lengthLabel.textContent = "長度：";
          lengthLabel.htmlFor = `${effectId}-shape-length`;

          const lengthInput = document.createElement("input");
          lengthInput.type = "number";
          lengthInput.id = `${effectId}-shape-length`;
          lengthInput.name = "shape_length";
          lengthInput.min = "1";
          lengthInput.value = "1";

          lengthField.appendChild(lengthLabel);
          lengthField.appendChild(lengthInput);
          shapeParamsContainer.appendChild(lengthField);
        } else if (shapeType === "cone") {
          // 長度
          const lengthField = document.createElement("div");
          lengthField.className = "field";

          const lengthLabel = document.createElement("label");
          lengthLabel.textContent = "長度：";
          lengthLabel.htmlFor = `${effectId}-shape-cone-length`;

          const lengthInput = document.createElement("input");
          lengthInput.type = "number";
          lengthInput.id = `${effectId}-shape-cone-length`;
          lengthInput.name = "shape_cone_length";
          lengthInput.min = "1";
          lengthInput.value = "1";

          lengthField.appendChild(lengthLabel);
          lengthField.appendChild(lengthInput);
          shapeParamsContainer.appendChild(lengthField);

          // 角度
          const angleField = document.createElement("div");
          angleField.className = "field";

          const angleLabel = document.createElement("label");
          angleLabel.textContent = "角度：";
          angleLabel.htmlFor = `${effectId}-shape-angle`;

          const angleInput = document.createElement("input");
          angleInput.type = "number";
          angleInput.id = `${effectId}-shape-angle`;
          angleInput.name = "shape_angle";
          angleInput.min = "0";
          angleInput.max = "360";
          angleInput.step = "0.1";
          angleInput.value = "60";

          angleField.appendChild(angleLabel);
          angleField.appendChild(angleInput);
          shapeParamsContainer.appendChild(angleField);
        }
      }
    });

    const shapeOptions = [
      { value: "point", text: "點" },
      { value: "circle", text: "圓形" },
      { value: "rectangle", text: "矩形" },
      { value: "line", text: "直線" },
      { value: "cone", text: "扇形" },
    ];

    shapeOptions.forEach((option) => {
      const optionElement = document.createElement("option");
      optionElement.value = option.value;
      optionElement.textContent = option.text;
      shapeSelect.appendChild(optionElement);
    });

    shapeField.appendChild(shapeLabel);
    shapeField.appendChild(shapeSelect);
    effectFields.appendChild(shapeField);

    // 形狀參數容器
    const shapeParamsContainer = document.createElement("div");
    shapeParamsContainer.className = "shape-params";
    effectFields.appendChild(shapeParamsContainer);

    // 根據效果類型添加不同的欄位
    if (type === "hp") {
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

    // 如果有現有的形狀數據，設置為選中
    if (shape && shape.type && typeof shape.type === "string") {
      // 先記錄形狀類型及其小寫形式，避免重複調用
      const shapeType = shape.type;
      const shapeTypeLower = shapeType.toLowerCase();
      console.log("形狀類型:", shapeType, "小寫:", shapeTypeLower);

      // 設置形狀類型
      const shapeTypeOption = Array.from(shapeSelect.options).find(
        (option) => option.value === shapeTypeLower
      );
      if (shapeTypeOption) {
        shapeTypeOption.selected = true;
      } else {
        console.warn("找不到對應的形狀類型選項:", shapeType);
      }
    } else if (shape) {
      console.warn("形狀數據無效或缺少類型屬性:", shape);
    }

    // 觸發形狀變更事件以初始化參數
    shapeSelect.dispatchEvent(new Event("change"));

    // 如果有現有的形狀數據，設置參數值
    if (shape && shape.type && typeof shape.type === "string") {
      // 等待 DOM 更新
      setTimeout(() => {
        const shapeParamsContainer =
          effectElement.querySelector(".shape-params");
        if (!shapeParamsContainer) return;

        const shapeType = shape.type.toLowerCase();

        // 根據形狀類型設置參數
        if (shapeType === "circle" && shape.area) {
          const areaInput = shapeParamsContainer.querySelector(
            "input[name='shape_area']"
          );
          if (areaInput) areaInput.value = shape.area;
        } else if (shapeType === "rectangle") {
          const widthInput = shapeParamsContainer.querySelector(
            "input[name='shape_width']"
          );
          const heightInput = shapeParamsContainer.querySelector(
            "input[name='shape_height']"
          );
          if (widthInput && shape.width) widthInput.value = shape.width;
          if (heightInput && shape.height) heightInput.value = shape.height;
        } else if (shapeType === "line" && shape.length) {
          const lengthInput = shapeParamsContainer.querySelector(
            "input[name='shape_length']"
          );
          if (lengthInput) lengthInput.value = shape.length;
        } else if (shapeType === "cone") {
          const lengthInput = shapeParamsContainer.querySelector(
            "input[name='shape_cone_length']"
          );
          const angleInput = shapeParamsContainer.querySelector(
            "input[name='shape_angle']"
          );
          if (lengthInput && shape.length) lengthInput.value = shape.length;
          if (angleInput && shape.angle) angleInput.value = shape.angle;
        }
      }, 0);
    } else if (shape) {
      console.warn("設置形狀參數時發現無效的形狀類型:", shape);
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
      const targetType = element.querySelector(
        "select[name='target_type']"
      )?.value;

      // 收集形狀資料
      const shapeType = element.querySelector(
        "select[name='shape_type']"
      ).value;
      let shape = {
        type: shapeType,
      };

      // 根據形狀類型添加額外屬性
      switch (shapeType) {
        case "circle":
          shape.area = parseInt(
            element.querySelector("input[name='shape_area']")?.value || 1
          );
          break;
        case "rectangle":
          shape.width = parseInt(
            element.querySelector("input[name='shape_width']")?.value || 1
          );
          shape.height = parseInt(
            element.querySelector("input[name='shape_height']")?.value || 1
          );
          break;
        case "line":
          shape.length = parseInt(
            element.querySelector("input[name='shape_length']")?.value || 1
          );
          break;
        case "cone":
          shape.length = parseInt(
            element.querySelector("input[name='shape_cone_length']")?.value || 1
          );
          shape.angle = parseFloat(
            element.querySelector("input[name='shape_angle']")?.value || 60
          );
          break;
      }

      if (effectType === "hp") {
        const value = parseInt(
          element.querySelector("input[name='value']").value || 0
        );

        effects.push({
          type: "hp",
          target_type: targetType,
          shape: shape,
          value: value,
        });
      } else if (effectType === "burn") {
        const duration = parseInt(
          element.querySelector("input[name='duration']").value || 1
        );

        effects.push({
          type: "burn",
          target_type: targetType,
          shape: shape,
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
      cost,
      hit_rate: hitRate,
      crit_rate: critRate,
      effects,
    };

    try {
      // 獲取檔案路徑，優先使用 this.selectedFile，如果 skillsData 有 file_path 則使用
      const filePath =
        this.selectedFile || (this.skillsData && this.skillsData.file_path);
      if (!filePath) {
        throw new Error("找不到檔案路徑");
      }

      console.log("保存技能到:", filePath, "技能ID:", this.selectedSkill);
      await api.saveSkill(filePath, this.selectedSkill, skillData);

      // 更新本地資料，避免重新載入
      if (this.skillsData && this.skillsData.skills) {
        // 直接替換整個技能對象，確保 effects 數組被完整保留
        this.skillsData.skills[this.selectedSkill] = skillData;
        console.log("本地技能數據已更新:", this.skillsData);

        // 只更新技能列表，不重新載入整個資料
        this.updateSkillList();
      } else {
        // 如果出現問題，仍然可以回退到完全重新載入
        console.warn("本地技能數據無效，重新載入中...");
        await this.loadSkills(filePath);
      }

      // 確保選中的技能仍然選中
      this.selectSkill(this.selectedSkill);

      // 顯示成功訊息
      alert("保存成功!");
    } catch (error) {
      console.error("保存技能時出錯:", error);
      alert(`保存失敗: ${error}`);
    }
  }
}

// 在 DOM 載入完成後初始化技能編輯器
let skillEditor;
document.addEventListener("DOMContentLoaded", () => {
  skillEditor = new SkillEditor();
});
