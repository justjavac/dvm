Deno.serve(async (req: Request) => {
  const userAgent = req.headers.get("User-Agent") || "";

  if (userAgent.includes("PowerShell")) {
    return new Response(await Deno.readFile("./install.ps1"));
  }

  if (userAgent.includes("curl")) {
    return new Response(await Deno.readFile("./install.sh"));
  }

  return new Response(`
  Deno Version Manager - Easy way to manage multiple active deno versions.

  Install With Shell:

    curl -fsSL https://raw.githubusercontent.com/justjavac/dvm/main/install.sh | sh
  
  Install With PowerShell:

    irm https://raw.githubusercontent.com/justjavac/dvm/main/install.ps1 | iex
  `);
});
