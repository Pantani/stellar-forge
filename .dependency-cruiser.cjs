/** @type {import("dependency-cruiser").IConfiguration} */
module.exports = {
  forbidden: [
    {
      name: "no-circular",
      severity: "error",
      comment: "Modules in the analyzed JS/TS surfaces must not depend on each other cyclically.",
      from: {
        pathNot: "^node_modules",
      },
      to: {
        circular: true,
      },
    },
    {
      name: "not-to-unresolvable",
      severity: "error",
      comment: "Every import in the analyzed JS/TS surfaces must resolve to a file, a core module, or an installed package.",
      from: {},
      to: {
        couldNotResolve: true,
      },
    },
    {
      name: "api-lib-is-foundation",
      severity: "error",
      comment: "The API lib layer is shared foundation code and must not depend on delivery, service, worker, or server entrypoint modules.",
      from: {
        path: "^demo/apps/api/src/lib/",
      },
      to: {
        path: "^demo/apps/api/src/(routes|services|workers)/|^demo/apps/api/src/server\\.ts$",
      },
    },
    {
      name: "api-services-stay-below-delivery",
      severity: "error",
      comment: "API services may depend on lib modules, but not on HTTP routes, workers, or the server composition root.",
      from: {
        path: "^demo/apps/api/src/services/",
      },
      to: {
        path: "^demo/apps/api/src/(routes|workers)/|^demo/apps/api/src/server\\.ts$",
      },
    },
    {
      name: "api-routes-do-not-depend-on-runtime-entrypoints",
      severity: "error",
      comment: "HTTP route modules are delivery adapters and must not import the server entrypoint or worker entrypoints.",
      from: {
        path: "^demo/apps/api/src/routes/",
      },
      to: {
        path: "^demo/apps/api/src/workers/|^demo/apps/api/src/server\\.ts$",
      },
    },
    {
      name: "api-workers-do-not-depend-on-delivery",
      severity: "error",
      comment: "Worker entrypoints may use lib modules, but must not depend on HTTP route modules or the server entrypoint.",
      from: {
        path: "^demo/apps/api/src/workers/",
      },
      to: {
        path: "^demo/apps/api/src/routes/|^demo/apps/api/src/server\\.ts$",
      },
    },
    {
      name: "api-does-not-depend-on-web",
      severity: "error",
      comment: "The API demo surface must not import frontend modules.",
      from: {
        path: "^demo/apps/api/src/",
      },
      to: {
        path: "^demo/apps/web/src/",
      },
    },
    {
      name: "web-does-not-depend-on-api-internals",
      severity: "error",
      comment: "The web demo surface must not import API internals; cross-app data should come from generated artifacts or runtime APIs.",
      from: {
        path: "^demo/apps/web/src/",
      },
      to: {
        path: "^demo/apps/api/src/",
      },
    },
    {
      name: "web-generated-is-data-boundary",
      severity: "error",
      comment: "Generated web state must remain a leaf data boundary and must not import UI modules.",
      from: {
        path: "^demo/apps/web/src/generated/",
      },
      to: {
        path: "^demo/apps/web/src/(?!generated/)",
      },
    },
    {
      name: "root-scripts-do-not-depend-on-app-internals",
      severity: "error",
      comment: "Repository and demo helper scripts should orchestrate commands, not import API or web internals directly.",
      from: {
        path: "^(scripts/|demo/scripts/|demo/workers/)",
      },
      to: {
        path: "^demo/apps/(api|web)/src/",
      },
    },
  ],
  options: {
    tsConfig: {
      fileName: "tsconfig.depcruise.json",
    },
    doNotFollow: {
      path: "node_modules",
    },
    exclude: {
      path: "^(target|_workspace|node_modules)/",
    },
    combinedDependencies: true,
    tsPreCompilationDeps: true,
    enhancedResolveOptions: {
      extensions: [".ts", ".tsx", ".mjs", ".js", ".jsx", ".json"],
    },
  },
};
