import React from "react";
import { Navbar } from "./components/Navbar";
import { Hero } from "./components/Hero";
import { InteractiveTerminal } from "./components/InteractiveTerminal";
import { BenchmarkTable } from "./components/BenchmarkTable";
import { ArchitectureMatrix } from "./components/ArchitectureMatrix";
import { EcosystemGrid } from "./components/EcosystemGrid";
import { InstallGuide } from "./components/InstallGuide";
import { Footer } from "./components/Footer";

export const App: React.FC = () => {
  return (
    <div style={{ minHeight: "100vh", display: "flex", flexDirection: "column" }}>
      <a className="skip-link" href="#main-content">Skip to content</a>
      <Navbar />
      <main id="main-content" style={{ flex: 1 }}>
        <Hero />
        <InteractiveTerminal />
        <BenchmarkTable />
        <ArchitectureMatrix />
        <EcosystemGrid />
        <InstallGuide />
      </main>
      <Footer />
    </div>
  );
};

export default App;
