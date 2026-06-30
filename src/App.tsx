import { BrowserRouter, Routes, Route } from "react-router-dom";
import { I18nProvider } from "@/i18n";
import { Layout } from "@/components/layout/Layout";
import { DashboardPage } from "@/pages/DashboardPage";
import { CleanPage } from "@/pages/CleanPage";
import { UninstallPage } from "@/pages/UninstallPage";
import { PurgePage } from "@/pages/PurgePage";
import { OptimizePage } from "@/pages/OptimizePage";
import { AnalyzePage } from "@/pages/AnalyzePage";
import { HistoryPage } from "@/pages/HistoryPage";
import { SettingsPage } from "@/pages/SettingsPage";

function App() {
  return (
    <I18nProvider>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/clean" element={<CleanPage />} />
          <Route path="/uninstall" element={<UninstallPage />} />
          <Route path="/purge" element={<PurgePage />} />
          <Route path="/optimize" element={<OptimizePage />} />
          <Route path="/analyze" element={<AnalyzePage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </I18nProvider>
  );
}

export default App;
