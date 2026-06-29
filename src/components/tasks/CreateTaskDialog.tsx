import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import { Input, TextArea } from "../ui/Input";
import { Button } from "../ui/Button";
import { useTaskStore } from "../../stores/taskStore";

interface CreateTaskDialogProps {
  open: boolean;
  onClose: () => void;
  repositoryId: string;
}

export function CreateTaskDialog({
  open,
  onClose,
  repositoryId,
}: CreateTaskDialogProps) {
  const { t } = useTranslation();
  const createTask = useTaskStore((s) => s.createTask);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;

    setLoading(true);
    try {
      await createTask(repositoryId, title.trim(), description.trim() || undefined);
      setTitle("");
      setDescription("");
      onClose();
    } catch {
      // error logged in store
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={onClose} title={t("createTask.title")}>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <Input
          id="task-title"
          label={t("createTask.name")}
          placeholder={t("createTask.namePlaceholder")}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          autoFocus
          required
        />
        <TextArea
          id="task-description"
          label={t("createTask.description")}
          placeholder={t("createTask.descriptionPlaceholder")}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={3}
        />
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" type="button" onClick={onClose}>
            {t("createTask.cancel")}
          </Button>
          <Button type="submit" disabled={!title.trim() || loading}>
            {t("createTask.create")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
