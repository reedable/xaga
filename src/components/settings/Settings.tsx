import { List, ListItemButton, ListItemText } from "@mui/material";
import { useEffect } from "react";
import {
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from "react-router-dom";
import { AppearanceSettings } from "./AppearanceSettings";
import { GeneralSettings } from "./GeneralSettings";
import css from "./_.module.css";

const SECTIONS = [
  { path: "general", label: "General" },
  { path: "appearance", label: "Appearance" },
];

export function Settings() {
  const navigate = useNavigate();
  const location = useLocation();
  const activeSection = location.pathname.split("/").pop();

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") navigate("/");
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [navigate]);

  return (
    <div className={css.Settings}>
      <nav className={css.Sidebar}>
        <List disablePadding>
          {SECTIONS.map(({ path, label }) => (
            <ListItemButton
              key={path}
              selected={activeSection === path}
              onClick={() => navigate(`/settings/${path}`)}
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
