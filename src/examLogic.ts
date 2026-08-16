import type { ExamAttempt } from "./types";

export function remainingExamSeconds(endsAt: string, now = Date.now()) {
  return Math.max(0, Math.ceil((new Date(endsAt).getTime() - now) / 1000));
}

export function formatExamTime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainder = seconds % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`
    : `${minutes}:${String(remainder).padStart(2, "0")}`;
}

export function examQuestionNumbers(attempt: Pick<ExamAttempt, "questionCount">) {
  const count = attempt.questionCount > 0 ? attempt.questionCount : 40;
  return Array.from({ length: count }, (_, index) => index + 1);
}

export function answeredQuestionCount(attempt: Pick<ExamAttempt, "answers">) {
  return new Set(attempt.answers.filter((answer) => answer.mcqChoice).map((answer) => answer.questionNumber)).size;
}
