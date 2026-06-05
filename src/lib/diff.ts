export interface DiffLine {
  type: "same" | "added" | "removed";
  text: string;
}

/** LCS-based line diff algorithm */
export function computeDiff(oldText: string, newText: string): DiffLine[] {
  const oldLines = (oldText || "").split("\n");
  const newLines = (newText || "").split("\n");
  const m = oldLines.length;
  const n = newLines.length;

  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (oldLines[i - 1] === newLines[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  const result: DiffLine[] = [];
  let i = m, j = n;
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldLines[i - 1] === newLines[j - 1]) {
      result.push({ type: "same", text: oldLines[i - 1] });
      i--; j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      result.push({ type: "added", text: newLines[j - 1] });
      j--;
    } else {
      result.push({ type: "removed", text: oldLines[i - 1] });
      i--;
    }
  }

  return result.reverse();
}
