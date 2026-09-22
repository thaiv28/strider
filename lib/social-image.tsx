import { ImageResponse } from "next/og";

export const socialImageSize = { width: 1200, height: 630 };
export const socialImageContentType = "image/png";

export function socialImage(title: string, subtitle: string) {
  return new ImageResponse(
    (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          width: "100%",
          height: "100%",
          padding: "70px 80px",
          color: "#eae4d3",
          background: "linear-gradient(135deg, #15251e 0%, #31493a 70%, #526146 100%)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 20, fontSize: 32, letterSpacing: 5 }}>
          <span style={{ width: 34, height: 34, background: "#d1b66d", transform: "rotate(45deg)" }} />
          STRIDER
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
          <div style={{ width: 105, height: 5, background: "#d1b66d" }} />
          <div style={{ fontSize: 74, lineHeight: 1.08, fontWeight: 700, letterSpacing: -3 }}>
            {title}
          </div>
          <div style={{ color: "#d3dacd", fontSize: 29 }}>{subtitle}</div>
        </div>
      </div>
    ),
    socialImageSize,
  );
}
