import type { TaskStatus } from "../types";

export const STATUS_CONFIG: Record<
  TaskStatus,
  { label: string; color: string; bgColor: string }
> = {
  pending: {
    label: "Pendente",
    color: "text-text-muted",
    bgColor: "bg-text-muted/20",
  },
  in_progress: {
    label: "Em Progresso",
    color: "text-warning",
    bgColor: "bg-warning/20",
  },
  completed: {
    label: "Concluído",
    color: "text-success",
    bgColor: "bg-success/20",
  },
};
