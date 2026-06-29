import { LgePhasePipeline } from "./LgePhasePipeline";
import { LgeArtifactPanel } from "./LgeArtifactPanel";

export function LgeProcessView() {
  return (
    <div className="flex flex-1 overflow-hidden">
      {/* Left panel: phase pipeline and controls */}
      <div className="w-[55%] overflow-y-auto">
        <LgePhasePipeline />
      </div>
      {/* Right panel: artifact viewer */}
      <div className="w-[45%]">
        <LgeArtifactPanel />
      </div>
    </div>
  );
}
