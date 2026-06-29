import { useState, useEffect } from "react";
import { TopBar } from "./components/layout/TopBar";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusBar } from "./components/layout/StatusBar";
import { HealthCheck } from "./components/layout/HealthCheck";
import { StaleWorktreeAlert } from "./components/layout/StaleWorktreeAlert";
import { TaskList } from "./components/tasks/TaskList";
import { TaskDetail } from "./components/tasks/TaskDetail";
import { LgeProcessView } from "./components/lge/LgeProcessView";
import { useTaskStore } from "./stores/taskStore";
import { useLgeStore, initPlanningQueueListeners } from "./stores/lgeStore";
import { useDemoMode } from "./demo/useDemoMode";
import { useDemoUIStore } from "./demo/demoStore";

export default function App() {
  const [healthChecked, setHealthChecked] = useState(false);
  const selectedTaskId = useTaskStore((s) => s.selectedTaskId);
  const viewingTaskId = useLgeStore((s) => s.viewingTaskId);
  const toast = useDemoUIStore((s) => s.toast);

  useDemoMode();

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    initPlanningQueueListeners().then((fn) => { cleanup = fn; });
    return () => { cleanup?.(); };
  }, []);

  if (!healthChecked) {
    return <HealthCheck onDismiss={() => setHealthChecked(true)} />;
  }

  return (
    <div className="flex h-screen flex-col bg-bg-primary">
      <TopBar />
      <StaleWorktreeAlert />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        {viewingTaskId ? (
          <LgeProcessView />
        ) : selectedTaskId ? (
          <TaskDetail />
        ) : (
          <TaskList />
        )}
      </div>
      <StatusBar />

      {/* Demo: toast de status */}
      {toast && (
        <div className="pointer-events-none fixed bottom-12 left-1/2 z-50 -translate-x-1/2">
          <div className="rounded-lg border border-border bg-bg-surface px-5 py-3 text-sm font-medium text-text-primary shadow-xl">
            {toast}
          </div>
        </div>
      )}
    </div>
  );
}
