# Character

- pixel art 要點: https://www.youtube.com/watch?v=Z8earctNBxg
- 諸多免費圖片素材
  - https://itch.io/c/4458804/side-scroller-fantasy-character-pixel-sprites
- 不錯的免費角色基底
  - https://verygo1.itch.io/pixel-rpg-character-asset-pack-004

# Region

- 小到建築，大到大陸的產生器: https://watabou.github.io/

# Procedural Generation

## **泊松磁盤採樣 (Poisson Disk Sampling)**

- **原理**：確保每個物件間有最小距離，使用 dart throwing 或 Mitchell's best-candidate 演算法生成點集。
- **優點**：分布均勻自然，避免物件過度集中。
- **缺點**：計算較複雜，對於大範圍可能較慢。
- **適用**：生成森林、礦脈等需要間距的自然地形。

## **細胞自動機 (Cellular Automaton)**

- **原理**：用格子狀態（活/死）模擬演化，如 Conway's Game of Life 或洞穴生成規則（若鄰居數量 > 某值則變成牆壁）。
- **優點**：能生成有機、自然的洞穴/地形。
- **缺點**：難以精確控制最終結果。
- **適用**：生成洞穴、河流或有機地形。

## **Perlin/Simplex 雜訊 (Noise Functions)**

- **原理**：使用梯度雜訊函數生成連續值，根據閾值決定是否放置物件（例如高度 > 0.5 放樹）。
- **優點**：生成平滑、連續的地形變化。
- **缺點**：需要調整參數，結果較難預測。
- **適用**：生成丘陵、山脈或地形高度變化。

## **隨機行走/醉漢行走 (Random Walk/Drunkard's Walk)**

- **原理**：從起點開始隨機移動，每步在附近格子放置物件，模擬擴散。
- **優點**：能生成有機的河流或洞穴。
- **缺點**：結果隨機性高，難以保證路徑連通。
- **適用**：生成河流、裂谷或擴散型地形。

## **波函數坍縮 (Wave Function Collapse)**

- **原理**：基於範例學習規則，逐步坍縮可能狀態直到所有格子確定。
  - 挑選「熵」最低的格子（可能性最少的格子）
  - 從該格子的可能狀態中隨機選擇一個
  - 根據規則更新所有受影響鄰居的可能狀態
  - 直到所有格子都被確定
- **優點**：能生成複雜、有規則的圖案。
- **缺點**：實作複雜，需要訓練資料。
- **適用**：生成城市佈局或複雜地圖圖案。
