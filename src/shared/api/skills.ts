import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type Scope = "global" | "project";

export interface SkillMeta {
  dirName: string;
  name: string;
  description: string;
  enabled: boolean;
  valid: boolean;
  error: string | null;
}

export interface ProjectInfo {
  name: string;
  path: string;
  exists: boolean;
  skills: SkillMeta[];
}

export interface StoreState {
  root: string;
  global: SkillMeta[];
  projects: ProjectInfo[];
}

export interface SkillRef {
  scope: Scope;
  project?: string | null;
  dirName: string;
}

export const skillsApi = {
  getState: () => invoke<StoreState>("get_state"),
  sync: () => invoke<StoreState>("sync_all"),
  addProject: (path: string) => invoke<StoreState>("add_project", { path }),
  removeProject: (name: string) => invoke<StoreState>("remove_project", { name }),
  importGlobalSkill: (source: string) => invoke<StoreState>("import_global_skill", { source }),
  setEnabled: (skills: SkillRef[], enabled: boolean) =>
    invoke<StoreState>("set_skills_enabled", { skills, enabled }),
  deleteSkills: (skills: SkillRef[]) => invoke<StoreState>("delete_skills", { skills }),
};

export async function pickDirectory(title: string): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false, title });
  return typeof selected === "string" ? selected : null;
}
