import { ImageResponse } from "next/og";

// Apple touch icon (home-screen). Next only auto-detects apple-icon as a raster
// image, so we render the Star of the Dúnedain seal to PNG at build time via
// ImageResponse (no native image libs required).
export const size = { width: 180, height: 180 };
export const contentType = "image/png";

const STAR =
  "data:image/svg+xml;utf8," +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><path fill="#eae4d3" d="M16 3 L17.91 11.38 L25.19 6.81 L20.62 14.09 L29 16 L20.62 17.91 L25.19 25.19 L17.91 20.62 L16 29 L14.09 20.62 L6.81 25.19 L11.38 17.91 L3 16 L11.38 14.09 L6.81 6.81 L14.09 11.38 Z"/></svg>`,
  );

export default function AppleIcon() {
  return new ImageResponse(
    (
      <div
        style={{
          display: "flex",
          width: "100%",
          height: "100%",
          alignItems: "center",
          justifyContent: "center",
          background: "#223028",
        }}
      >
        <div
          style={{
            display: "flex",
            width: 150,
            height: 150,
            alignItems: "center",
            justifyContent: "center",
            borderRadius: "50%",
            border: "6px solid #9c8b5a",
          }}
        >
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={STAR} width={116} height={116} alt="" />
        </div>
      </div>
    ),
    size,
  );
}
