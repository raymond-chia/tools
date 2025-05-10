// 視圖管理
class ViewManager {
  constructor() {
    this.homeView = document.getElementById("home-view");
    this.editorView = document.getElementById("editor-view");
    this.homeBtn = document.getElementById("home-btn");
    this.editorBtn = document.getElementById("editor-btn");
    this.startBtn = document.getElementById("start-btn");

    this.initEventListeners();
  }

  initEventListeners() {
    // 首頁開始按鈕
    this.startBtn.addEventListener("click", () => this.showEditor());

    // 導航按鈕
    this.homeBtn.addEventListener("click", () => this.showHome());
    this.editorBtn.addEventListener("click", () => this.showEditor());
  }

  // 顯示首頁
  showHome() {
    this.homeView.classList.remove("hidden");
    this.editorView.classList.add("hidden");
    this.homeBtn.classList.add("active");
    this.editorBtn.classList.remove("active");
  }

  // 顯示編輯器
  showEditor() {
    this.homeView.classList.add("hidden");
    this.editorView.classList.remove("hidden");
    this.homeBtn.classList.remove("active");
    this.editorBtn.classList.add("active");
  }
}

// 在 DOM 載入完成後初始化視圖管理器
let viewManager;
document.addEventListener("DOMContentLoaded", () => {
  viewManager = new ViewManager();
});
