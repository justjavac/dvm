import { serve } from "https://deno.land/std@0.152.0/http/server.ts";

serve(async (req: Request) => {
  const userAgent = req.headers.get("User-Agent") || "";

  if (userAgent.includes("WindowsPowerShell")) {
    return new Response(await Deno.readFile("./install.ps1"));
  }

  if (userAgent.includes("curl")) {
    return new Response(await Deno.readFile("./install.sh"));
  }

  return new Response(`
  Deno Version Manager - Easy way to manage multiple active deno versions.

  Install With Shell:

    curl -fsSL https://dvm.deno.dev | sh
  
  Install With PowerShell:

    irm https://dvm.deno.dev | iex
  `);
});
