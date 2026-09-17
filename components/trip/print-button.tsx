"use client";

import { Printer } from "lucide-react";
import { Button } from "@/components/ui";

export function PrintButton() {
  return (
    <Button variant="outline" onClick={() => window.print()} className="no-print">
      <Printer size={16} /> <span className="ml-1.5">Print</span>
    </Button>
  );
}
