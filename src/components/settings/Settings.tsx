import { List, ListItemButton, ListItemText } from "@mui/material";
import {
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from "react-router-dom";
import { AppearanceSettings } from "./AppearanceSettings";
import { GeneralSettings } from "./GeneralSettings";
import css from "./Settings.module.css";

const SECTIONS = [
  { path: "general", label: "General" },
  { path: "appearance", label: "Appearance" },
];

export function Settings() {
  const navigate = useNavigate();
  const location = useLocation();
  const activeSection = location.pathname.split("/").pop();

  return (
    <div className={css.Settings}>
      <nav className={css.Sidebar}>
        <List disablePadding>
          {SECTIONS.map(({ path, label }) => (
            <ListItemButton
              key={path}
              selected={activeSection === path}
              onClick={() => navigate(`/${path}`)}
            >
              <ListItemText primary={label} />
            </ListItemButton>
          ))}
        </List>
      </nav>

      <div className={css.Layout}>
        <main className={css.Content}>
          <Routes>
            <Route index element={<Navigate to="general" replace />} />
            <Route path="general" element={<GeneralSettings />} />
            <Route path="appearance" element={<AppearanceSettings />} />
          </Routes>
        </main>
      </div>
    </div>
  );
}
