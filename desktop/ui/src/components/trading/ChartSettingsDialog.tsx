import { Dialog } from "@nest/components";

type ChartSettingsDialogProps = {
  open: boolean;
  onClose: () => void;
};

/** Minimal placeholder — full chart settings (candle style, session hours, etc.) aren't needed yet. */
export function ChartSettingsDialog({ open, onClose }: ChartSettingsDialogProps) {
  return (
    <Dialog open={open} onClose={onClose} title="Chart Settings">
      <div className="w-[400px] max-w-full py-2 text-[12px] text-nest-muted">
        More chart settings are coming later. Nothing configurable here yet.
      </div>
    </Dialog>
  );
}
