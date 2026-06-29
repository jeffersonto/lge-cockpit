import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface Props {
  mermaidSource: string;
}

// Lazy-loaded mermaid instance to keep it out of the main bundle
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let mermaidInstance: any | null = null;

async function getMermaid() {
  if (mermaidInstance) return mermaidInstance;
  const mod = await import("mermaid");
  mermaidInstance = mod.default;
  mermaidInstance.initialize({
    startOnLoad: false,
    theme: "dark",
    themeVariables: {
      primaryColor: "#7c3aed",
      primaryTextColor: "#f1f5f9",
      primaryBorderColor: "#7c3aed",
      lineColor: "#334155",
      secondaryColor: "#222240",
      tertiaryColor: "#1a1a2e",
      background: "#0f0f1a",
      mainBkg: "#222240",
      nodeBorder: "#334155",
      clusterBkg: "#1a1a2e",
      titleColor: "#f1f5f9",
      edgeLabelBackground: "#222240",
      fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
      fontSize: "12px",
    },
    securityLevel: "strict",
  });
  return mermaidInstance;
}

export function DependencyDiagramView({ mermaidSource }: Props) {
  const { t } = useTranslation();
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [zoom, setZoom] = useState(1);
  const [hasError, setHasError] = useState(false);
  const blobUrlRef = useRef<string | null>(null);

  useEffect(() => {
    if (!mermaidSource) {
      setImageUrl(null);
      setHasError(false);
      return;
    }

    const id = `mermaid-${Date.now()}`;
    getMermaid()
      .then((m) => m.render(id, mermaidSource))
      .then(({ svg }: { svg: string }) => {
        if (blobUrlRef.current) {
          URL.revokeObjectURL(blobUrlRef.current);
        }
        // Render SVG as a sandboxed image via blob URL — safe against XSS
        const blob = new Blob([svg], { type: "image/svg+xml" });
        const url = URL.createObjectURL(blob);
        blobUrlRef.current = url;
        setImageUrl(url);
        setHasError(false);
      })
      .catch((err: unknown) => {
        console.warn("Mermaid render error:", err);
        setImageUrl(null);
        setHasError(true);
      });

    return () => {
      if (blobUrlRef.current) {
        URL.revokeObjectURL(blobUrlRef.current);
        blobUrlRef.current = null;
      }
    };
  }, [mermaidSource]);

  if (!mermaidSource) {
    return (
      <p className="text-center text-sm text-text-muted py-4">
        {t("lge.artifacts.archDiff.depGraphEmpty")}
      </p>
    );
  }

  if (hasError) {
    return (
      <p className="text-center text-sm text-error py-4">
        Failed to render dependency diagram
      </p>
    );
  }

  return (
    <div>
      {imageUrl && (
        <div className="mb-2 flex items-center justify-end gap-2">
          <button
            onClick={() => setZoom((z) => Math.max(0.4, z - 0.2))}
            className="rounded border border-border px-2 py-0.5 text-xs text-text-muted hover:text-text-primary hover:bg-bg-hover"
          >
            −
          </button>
          <span className="text-xs text-text-muted w-8 text-center">{Math.round(zoom * 100)}%</span>
          <button
            onClick={() => setZoom((z) => Math.min(2.5, z + 0.2))}
            className="rounded border border-border px-2 py-0.5 text-xs text-text-muted hover:text-text-primary hover:bg-bg-hover"
          >
            +
          </button>
          <button
            onClick={() => setZoom(1)}
            className="rounded border border-border px-2 py-0.5 text-xs text-text-muted hover:text-text-primary hover:bg-bg-hover"
          >
            Reset
          </button>
        </div>
      )}

      <div className="overflow-x-auto rounded-lg border border-border bg-bg-surface p-4 min-h-[120px] flex items-start">
        {imageUrl ? (
          <img
            src={imageUrl}
            alt="Dependency graph"
            style={{
              transform: `scale(${zoom})`,
              transformOrigin: "top left",
              transition: "transform 0.15s ease",
              maxWidth: "none",
            }}
          />
        ) : (
          <div className="flex flex-1 items-center justify-center">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-accent border-t-transparent" />
          </div>
        )}
      </div>
    </div>
  );
}
