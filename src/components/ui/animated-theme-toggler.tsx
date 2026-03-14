import { useRef } from "react"
import { Moon, Sun } from "lucide-react"
import { cn } from "@/lib/utils"
import { useTheme } from "@/hooks/useTheme"

interface AnimatedThemeTogglerProps extends React.ComponentPropsWithoutRef<"button"> {
  duration?: number
}

export const AnimatedThemeToggler = ({
  className,
  ...props
}: AnimatedThemeTogglerProps) => {
  const { mode, toggleModeWithTransition } = useTheme()
  const isDark = mode === "dark"
  const buttonRef = useRef<HTMLButtonElement>(null)

  return (
    <button
      type="button"
      ref={buttonRef}
      onClick={(e) => toggleModeWithTransition(e)}
      className={cn(
        "group relative flex size-9 items-center justify-center rounded-xl border border-border/10 bg-background/20 backdrop-blur-xl transition-all duration-300 hover:bg-foreground/5 hover:border-border/20 active:scale-95 sm:rounded-2xl",
        className
      )}
      {...props}
    >
      <div className="relative size-4.5 overflow-hidden">
        <div className="flex h-full w-full flex-col transition-transform duration-500 ease-[cubic-bezier(0.23,1,0.32,1)]"
             style={{ transform: isDark ? 'translateY(-100%)' : 'translateY(0)' }}>
          <div className="flex size-4.5 shrink-0 items-center justify-center">
            <Sun className="size-4 text-amber-500 transition-all duration-300 group-hover:rotate-12" strokeWidth={2.5} />
          </div>
          <div className="flex size-4.5 shrink-0 items-center justify-center">
            <Moon className="size-4 text-blue-400 transition-all duration-300 group-hover:-rotate-12" strokeWidth={2.5} />
          </div>
        </div>
      </div>
      <span className="sr-only">Toggle theme</span>
    </button>
  )
}
