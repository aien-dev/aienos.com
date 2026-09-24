import React from "react";
import { BarChart3, ExternalLink } from "lucide-react";

export const BenchmarkTable: React.FC = () => {
  const measurements = [
    {
      name: "Continuous batching sweep",
      what: "TTFT, inter-token latency, and throughput across concurrency levels",
      cmd: "cargo run -p aien-scheduler --bin bench_inference_stack",
      status: "Regenerating"
    },
    {
      name: "KV-cache branch forking",
      what: "Copy-on-write fork latency and shared page accounting",
      cmd: "cargo test -p aien-kv-cache --test cow_branching_tests -- --nocapture",
      status: "Regenerating"
    },
    {
      name: "AIEN vs MAX serving",
      what: "Apples-to-apples comparison against Modular MAX serve",
      cmd: "cargo run -p bench_apples_to_apples -- --engines aien,max --concurrency 1,2,4,8,16,32,64",
      status: "Regenerating"
    },
    {
      name: "Service footprint comparison",
      what: "RSS, latency, and throughput versus a Python FastAPI baseline",
      cmd: "See aien-dev/benchmarks",
      status: "Withdrawn"
    }
  ];

  const bundle = [
    "Commit identity",
    "Hardware and environment record",
    "Exact command",
    "Raw samples",
    "SHA-256 digests",
    "Measurement definition",
    "Reproducibility steps"
  ];

  return (
    <section id="benchmarks" className="section section-alt">
      <div className="container">
        <div className="section-head">
          <div className="badge badge-green">
            <BarChart3 size={12} />
            <span>EVIDENCE STANDARD</span>
          </div>
          <h2>Measured Results</h2>
          <p>
            Every headline number must resolve to a reproducible command and an evidence artifact.
            Earlier published figures are withdrawn until their artifact bundles are regenerated on
            NVIDIA DGX Spark (Grace Blackwell GB10). Nothing below is a claim yet; it is the list of
            what is being measured and how to reproduce it.
          </p>
        </div>

        <div className="data-table-wrap">
          <table className="data-table">
            <thead>
              <tr>
                <th scope="col">Measurement</th>
                <th scope="col">Reproduce</th>
                <th scope="col">Status</th>
              </tr>
            </thead>
            <tbody>
              {measurements.map((m) => (
                <tr key={m.name}>
                  <td>
                    <div>{m.name}</div>
                    <div style={{ fontWeight: 400, fontSize: "13px", color: "var(--text-secondary)", marginTop: "4px" }}>
                      {m.what}
                    </div>
                  </td>
                  <td data-label="Reproduce">
                    <code>{m.cmd}</code>
                  </td>
                  <td data-label="Status" style={{ whiteSpace: "nowrap" }}>
                    <span className={m.status === "Withdrawn" ? "badge badge-muted" : "badge badge-amber"}>
                      {m.status}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div style={{
          marginTop: "24px",
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: "8px"
        }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--text-muted)", marginRight: "4px", letterSpacing: "0.06em" }}>
            EACH BUNDLE CARRIES
          </span>
          {bundle.map((b) => (
            <span key={b} className="badge badge-muted" style={{ textTransform: "none", letterSpacing: 0 }}>{b}</span>
          ))}
        </div>

        <a
          href="https://github.com/aien-dev/benchmarks"
          target="_blank"
          rel="noopener noreferrer"
          className="prose-link"
          style={{ display: "inline-flex", alignItems: "center", gap: "6px", marginTop: "20px", fontSize: "14px" }}
        >
          Evidence artifacts: aien-dev/benchmarks
          <ExternalLink size={13} />
        </a>
      </div>
    </section>
  );
};
