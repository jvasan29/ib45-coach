export const CORE_MATRIX: Record<string, Record<string, number>> = {
  A: { A: 3, B: 3, C: 2, D: 2, E: 0 },
  B: { A: 3, B: 2, C: 2, D: 1, E: 0 },
  C: { A: 2, B: 2, C: 1, D: 0, E: 0 },
  D: { A: 2, B: 1, C: 0, D: 0, E: 0 },
  E: { A: 0, B: 0, C: 0, D: 0, E: 0 },
};

export function calculateCorePoints(ee: string, tok: string) {
  return CORE_MATRIX[ee]?.[tok] ?? 0;
}

export function calculateProjection(subjectGrades: number[], ee: string, tok: string, confidence: number) {
  const subjectPoints = subjectGrades.reduce((sum, grade) => sum + Math.max(1, Math.min(7, grade)), 0);
  const corePoints = calculateCorePoints(ee, tok);
  const totalPoints = subjectPoints + corePoints;
  const spread = Math.ceil((1 - Math.max(0, Math.min(1, confidence))) * 5);
  return {
    subjectPoints,
    corePoints,
    totalPoints,
    low: Math.max(0, totalPoints - spread),
    high: Math.min(45, totalPoints + spread),
    targetGap: Math.max(0, 45 - totalPoints),
  };
}
