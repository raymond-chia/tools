// 主要邏輯和頁面導航
document.addEventListener("DOMContentLoaded", () => {
  // 視圖切換
  const homeView = document.getElementById("home-view");
  const editorView = document.getElementById("editor-view");
  const homeBtn = document.getElementById("home-btn");
  const editorBtn = document.getElementById("editor-btn");
  const startBtn = document.getElementById("start-btn");

  // 切換到首頁
  function showHome() {
    homeView.classList.remove("hidden");
    editorView.classList.add("hidden");
    homeBtn.classList.add("active");
    editorBtn.classList.remove("active");
  }

  // 切換到編輯器
  function showEditor() {
    homeView.classList.add("hidden");
    editorView.classList.remove("hidden");
    homeBtn.classList.remove("active");
    editorBtn.classList.add("active");
  }

  // 設置事件監聽器
  homeBtn.addEventListener("click", showHome);
  editorBtn.addEventListener("click", showEditor);
  startBtn.addEventListener("click", showEditor);

  // 預設顯示首頁
  showHome();
});
