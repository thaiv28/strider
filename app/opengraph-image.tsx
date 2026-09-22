import { socialImage, socialImageContentType, socialImageSize } from "@/lib/social-image";

export const size = socialImageSize;
export const contentType = socialImageContentType;
export const alt = "Strider backpacking trip planner";

export default function OpenGraphImage() {
  return socialImage("Plan farther. Pack smarter.", "Routes, gear, food, and every detail of the journey.");
}
