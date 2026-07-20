import { useEffect, useState, type ReactNode } from "react";
import { WindowControls } from "./WindowControls";
import { isTauri } from "../lib/tauri";
import type { ThemeSummary } from "../lib/nest";

type TitleBarMenu = "file" | "developer" | "help" | null;
type FileSubmenu = "theme" | null;

type TitleBarProps = {
  /** Centered window title (usually the app's display name). */
  title: string;
  /** File → Quit. */
  onQuit: () => void;
  /** Developer → Show loaded recipes. */
  onShowRecipes: () => void;
  /** Help → About. */
  onAbout: () => void;
  /** File → Theme: every registered theme, for the submenu list. */
  themes: ThemeSummary[];
  /** Id of the currently active theme, for the submenu's checkmark. */
  activeThemeId: string | null;
  /** File → Theme → <pick one>. */
  onSelectTheme: (id: string) => void;
};

const menuButtonClass = "h-full px-2.5 text-[12px] text-nest-foreground hover:bg-nest-muted/12";

const menuDropdownClass =
  "absolute left-0 top-full z-[80] min-w-48 rounded-nest-md border border-nest-border bg-nest-background py-1 shadow-lg";

const menuItemClass = "flex w-full items-center px-3 py-1.5 text-left text-[12px] hover:bg-nest-muted/10";

const menuItemDisabledClass =
  "flex w-full cursor-default items-center px-3 py-1.5 text-left text-[12px] text-nest-muted/50";

/** Converts a kebab-case theme id (`cursor-dark`) to a display label (`Cursor Dark`). */
function formatThemeLabel(id: string): string {
  return id
    .split("-")
    .map((word) => (word.length > 0 ? word[0]!.toUpperCase() + word.slice(1) : word))
    .join(" ");
}

/**
 * Frameless title bar: File/Developer/Help menus, centered app title, window
 * controls. Pairs with `"decorations": false` in `tauri.conf.json`.
 */
export function TitleBar({
  title,
  onQuit,
  onShowRecipes,
  onAbout,
  themes,
  activeThemeId,
  onSelectTheme,
}: TitleBarProps) {
  const [openMenu, setOpenMenu] = useState<TitleBarMenu>(null);
  const [fileSubmenu, setFileSubmenu] = useState<FileSubmenu>(null);
  const close = () => {
    setOpenMenu(null);
    setFileSubmenu(null);
  };
  const showWindowChrome = isTauri();

  useEffect(() => {
    if (!openMenu) {
      return;
    }
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }
      if (target.closest("[data-titlebar-menu]")) {
        return;
      }
      close();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };
    window.addEventListener("mousedown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mousedown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [openMenu]);

  const toggleMenu = (menu: Exclude<TitleBarMenu, null>) => {
    setOpenMenu((current) => (current === menu ? null : menu));
    setFileSubmenu(null);
  };

  const toggleFileSubmenu = (submenu: Exclude<FileSubmenu, null>) => {
    setFileSubmenu((current) => (current === submenu ? null : submenu));
  };

  return (
    <header className="relative flex h-8 shrink-0 items-stretch border-b border-nest-border bg-nest-surface text-[13px]">
      <div className="relative z-10 flex h-full shrink-0 items-stretch pl-2" data-titlebar-menu>
        <MenuDropdown label="File" open={openMenu === "file"} onToggle={() => toggleMenu("file")}>
          {themes.length > 0 ? (
            <MenuSubmenu
              label="Theme"
              open={fileSubmenu === "theme"}
              onToggle={() => toggleFileSubmenu("theme")}
            >
              {themes.map((theme) => (
                <MenuRadioItem
                  key={theme.id}
                  label={formatThemeLabel(theme.id)}
                  checked={theme.id === activeThemeId}
                  onClick={() => {
                    onSelectTheme(theme.id);
                    close();
                  }}
                />
              ))}
            </MenuSubmenu>
          ) : (
            <MenuItemDisabled label="Theme" />
          )}
          <MenuItem
            label="Quit"
            onClick={() => {
              onQuit();
              close();
            }}
          />
        </MenuDropdown>

        <MenuDropdown
          label="Developer"
          open={openMenu === "developer"}
          onToggle={() => toggleMenu("developer")}
        >
          <MenuItem
            label="Show loaded recipes"
            onClick={() => {
              onShowRecipes();
              close();
            }}
          />
        </MenuDropdown>

        <MenuDropdown label="Help" open={openMenu === "help"} onToggle={() => toggleMenu("help")}>
          <MenuItem
            label="About"
            onClick={() => {
              onAbout();
              close();
            }}
          />
        </MenuDropdown>
      </div>

      {showWindowChrome ? (
        <div className="min-w-0 flex-1" data-tauri-drag-region />
      ) : (
        <div className="min-w-0 flex-1" />
      )}

      <div className="relative z-10 flex h-full shrink-0 items-stretch">
        {showWindowChrome ? <WindowControls /> : null}
      </div>

      <p
        className="pointer-events-none absolute inset-0 z-0 flex items-center justify-center px-28"
        aria-hidden
      >
        <span className="truncate text-[12px] font-medium text-nest-foreground">{title}</span>
      </p>
    </header>
  );
}

function MenuDropdown({
  label,
  open,
  onToggle,
  children,
}: {
  label: string;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  return (
    <div className="relative flex h-full items-stretch">
      <button type="button" className={menuButtonClass} onClick={onToggle}>
        {label}
      </button>
      {open ? (
        <div className={menuDropdownClass} role="menu" data-titlebar-menu>
          {children}
        </div>
      ) : null}
    </div>
  );
}

function MenuItem({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button type="button" role="menuitem" className={menuItemClass} onClick={onClick}>
      {label}
    </button>
  );
}

function MenuItemDisabled({ label }: { label: string }) {
  return (
    <span role="menuitem" aria-disabled className={menuItemDisabledClass}>
      {label}
    </span>
  );
}

/**
 * A nested menu that expands to the right, e.g. File → Theme. Opens/closes
 * on click (not hover) — a hover-based (mouseenter/mouseleave) submenu is
 * fragile: moving the pointer at an angle from the trigger toward an item
 * can register as leaving the hover zone before the click lands, collapsing
 * the menu out from under the click. Click avoids that timing dependency
 * entirely.
 */
function MenuSubmenu({
  label,
  open,
  onToggle,
  children,
}: {
  label: string;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  return (
    <div className="relative">
      <button
        type="button"
        role="menuitem"
        aria-haspopup="menu"
        aria-expanded={open}
        className={`${menuItemClass} justify-between gap-4`}
        onClick={onToggle}
      >
        <span>{label}</span>
        <span aria-hidden className="text-nest-muted">
          {"›"}
        </span>
      </button>
      {open ? (
        <div className={`${menuDropdownClass} left-full top-0`} role="menu" data-titlebar-menu>
          {children}
        </div>
      ) : null}
    </div>
  );
}

/** A selectable menu item showing a checkmark when active, e.g. a theme choice. */
function MenuRadioItem({
  label,
  checked,
  onClick,
}: {
  label: string;
  checked: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitemradio"
      aria-checked={checked}
      className={`${menuItemClass} justify-between gap-4`}
      onClick={onClick}
    >
      <span>{label}</span>
      {checked ? <span aria-hidden>{"✓"}</span> : null}
    </button>
  );
}
