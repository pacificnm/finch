import { config, library } from "@fortawesome/fontawesome-svg-core";
import {
  faBars,
  faCalendar,
  faChevronDown,
  faChevronLeft,
  faChevronRight,
  faChevronUp,
  faCircleCheck,
  faCircleExclamation,
  faCircleInfo,
  faDoorOpen,
  faGear,
  faMagnifyingGlass,
  faMinus,
  faPlus,
  faRotateRight,
  faTrash,
  faTriangleExclamation,
  faWindowMaximize,
  faWindowRestore,
  faXmark,
} from "@fortawesome/free-solid-svg-icons";

import "@fortawesome/fontawesome-svg-core/styles.css";

// We add CSS manually via the imported stylesheet; disable auto-injection to
// avoid the flash of oversized icons before the app stylesheet loads.
config.autoAddCss = false;

library.add(
  faBars,
  faCalendar,
  faChevronDown,
  faChevronLeft,
  faChevronRight,
  faChevronUp,
  faCircleCheck,
  faCircleExclamation,
  faCircleInfo,
  faDoorOpen,
  faGear,
  faMagnifyingGlass,
  faMinus,
  faPlus,
  faRotateRight,
  faTrash,
  faTriangleExclamation,
  faWindowMaximize,
  faWindowRestore,
  faXmark,
);

export {
  faBars,
  faCalendar,
  faChevronDown,
  faChevronLeft,
  faChevronRight,
  faChevronUp,
  faCircleCheck,
  faCircleExclamation,
  faCircleInfo,
  faDoorOpen,
  faGear,
  faMagnifyingGlass,
  faMinus,
  faPlus,
  faRotateRight,
  faTrash,
  faTriangleExclamation,
  faWindowMaximize,
  faWindowRestore,
  faXmark,
};
