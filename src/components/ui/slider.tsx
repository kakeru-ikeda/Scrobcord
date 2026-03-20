import { cn } from "../../lib/utils";

interface SliderProps {
  min: number;
  max: number;
  step?: number;
  value: number;
  onValueChange: (value: number) => void;
  disabled?: boolean;
  className?: string;
}

function Slider({ min, max, step = 1, value, onValueChange, disabled, className }: SliderProps) {
  return (
    <input
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      disabled={disabled}
      onChange={(e) => onValueChange(Number(e.target.value))}
      className={cn(
        "w-full h-2 rounded-full appearance-none cursor-pointer",
        "bg-muted accent-primary",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className
      )}
    />
  );
}

export { Slider };
