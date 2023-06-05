import "./App.css";
import { AppSidebar } from "./components/navigation/AppSidebar";
import AppStore from "./store/AppStore";
import SettingsStore from "./store/SettingsStore";

function App() {
  const { settings, setGlobalHotkeyEvent, globalHotkeyEvent } = SettingsStore;
  const { sidebarIcons, setSidebarIcon } = AppStore;

  const sIcon = sidebarIcons().find((icon) => icon.current);

  return (
    <div class="dark:bg-dark absolute flex h-full w-full overflow-hidden bg-white text-black dark:text-white ">
      <div class="dark:bg-dark-light flex flex-col items-center space-y-7 bg-gray-200 px-3.5 pt-5">
        <AppSidebar />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex w-full justify-between py-1 pl-2">
          <p class="dark:bg-dark-dark bg-gray-50 text-xs font-semibold text-gray-500 dark:text-white ">
            {sIcon?.name?.toLocaleUpperCase()}
          </p>
          <FontAwesomeIcon
            icon={settings?.synchronize ? ["fas", "globe"] : ["far", "hdd"]}
            title={settings?.synchronize ? "online" : "offline"}
            class="text-1xl mr-2"
          />
        </div>
        {sIcon?.name === "Recent Clipboards" && sIcon?.current && (
          <RecentClipboards />
        )}
        {sIcon?.name === "Starred Clipboards" && sIcon?.current && (
          <StarredClipboards />
        )}
        {sIcon?.name === "History" && sIcon?.current && <History />}
        {sIcon?.name === "View more" && sIcon?.current && <ViewMore />}
      </div>
    </div>
  );
}

export default App;
