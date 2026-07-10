import { AlertTriangle, Check, FolderPlus, Plus, RefreshCw, Trash2, X } from "lucide-react";
import { useMemo } from "react";

import {
  type ProjectInfo,
  pickDirectory,
  type SkillMeta,
  type SkillRef,
  type StoreState,
  skillsApi,
} from "@/shared/api/skills";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/button";
import { Checkbox } from "@/shared/ui/checkbox";
import { Switch } from "@/shared/ui/switch";

import { skillKey, useSkillsStore } from "./useSkillsStore";

export function SkillsManager() {
  const { state, selected, busy, error, run, toggleSelect, clearSelection } = useSkillsStore();

  const globalRefs = useMemo<SkillRef[]>(
    () =>
      (state?.global ?? []).map((s) => ({
        scope: "global",
        dirName: s.dirName,
      })),
    [state],
  );

  const projectRefs = useMemo<SkillRef[]>(
    () =>
      (state?.projects ?? []).flatMap((p) =>
        p.skills.map((s) => ({
          scope: "project" as const,
          project: p.name,
          dirName: s.dirName,
        })),
      ),
    [state],
  );

  const selectedFrom = (refs: SkillRef[]) => refs.filter((r) => selected.has(skillKey(r)));

  const mutate = async (fn: () => Promise<StoreState>) => {
    await run(fn);
    clearSelection();
  };

  const onToggleEnabled = (ref: SkillRef, enabled: boolean) =>
    run(() => skillsApi.setEnabled([ref], enabled));

  const bulkEnable = (refs: SkillRef[], enabled: boolean) => {
    if (refs.length === 0) return;
    void mutate(() => skillsApi.setEnabled(refs, enabled));
  };

  const bulkDelete = (refs: SkillRef[]) => {
    if (refs.length === 0) return;
    const ok = window.confirm(`Удалить ${refs.length} скилл(ов) из хранилища безвозвратно?`);
    if (!ok) return;
    void mutate(() => skillsApi.deleteSkills(refs));
  };

  const onAddGlobal = async () => {
    const dir = await pickDirectory("Выберите папку скилла (с SKILL.md)");
    if (dir) void run(() => skillsApi.importGlobalSkill(dir));
  };

  const onAddProject = async () => {
    const dir = await pickDirectory("Выберите папку проекта");
    if (dir) void run(() => skillsApi.addProject(dir));
  };

  const onRemoveProject = (name: string) => {
    if (!window.confirm(`Убрать проект «${name}» из хранилища?`)) return;
    void mutate(() => skillsApi.removeProject(name));
  };

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <header className="flex items-center justify-between gap-3 border-b px-4 py-3">
        <div className="min-w-0">
          <h1 className="font-heading font-semibold text-base">Skills Store</h1>
          <p className="truncate text-muted-foreground text-xs">{state?.root ?? "…"}</p>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={busy}
          onClick={() => run(() => skillsApi.sync())}
        >
          <RefreshCw className={cn(busy && "animate-spin")} />
          Sync
        </Button>
      </header>

      {error && (
        <div className="flex items-center gap-2 border-destructive/30 border-b bg-destructive/10 px-4 py-2 text-destructive text-xs">
          <AlertTriangle className="size-4" />
          <span className="truncate">{error}</span>
        </div>
      )}

      <div className="grid min-h-0 flex-1 grid-cols-2 divide-x">
        <Column
          title="Global"
          count={state?.global.length ?? 0}
          onAdd={onAddGlobal}
          addIcon={<Plus />}
          onEnable={() => bulkEnable(selectedFrom(globalRefs), true)}
          onDisable={() => bulkEnable(selectedFrom(globalRefs), false)}
          onDelete={() => bulkDelete(selectedFrom(globalRefs))}
          selectedCount={selectedFrom(globalRefs).length}
          busy={busy}
        >
          {(state?.global ?? []).map((skill) => {
            const ref: SkillRef = { scope: "global", dirName: skill.dirName };
            return (
              <SkillRow
                key={skill.dirName}
                skill={skill}
                selected={selected.has(skillKey(ref))}
                busy={busy}
                onToggleSelect={() => toggleSelect(ref)}
                onToggleEnabled={(v) => onToggleEnabled(ref, v)}
              />
            );
          })}
          {state && state.global.length === 0 && (
            <EmptyHint text="Нет глобальных скиллов. Нажмите Sync или + чтобы импортировать." />
          )}
        </Column>

        <Column
          title="Projects"
          count={state?.projects.length ?? 0}
          onAdd={onAddProject}
          addIcon={<FolderPlus />}
          onEnable={() => bulkEnable(selectedFrom(projectRefs), true)}
          onDisable={() => bulkEnable(selectedFrom(projectRefs), false)}
          onDelete={() => bulkDelete(selectedFrom(projectRefs))}
          selectedCount={selectedFrom(projectRefs).length}
          busy={busy}
        >
          {(state?.projects ?? []).map((project) => (
            <ProjectGroup
              key={project.name}
              project={project}
              selected={selected}
              busy={busy}
              onToggleSelect={toggleSelect}
              onToggleEnabled={onToggleEnabled}
              onRemove={() => onRemoveProject(project.name)}
            />
          ))}
          {state && state.projects.length === 0 && (
            <EmptyHint text="Нет проектов. Нажмите + чтобы добавить папку проекта." />
          )}
        </Column>
      </div>
    </div>
  );
}

function Column(props: {
  title: string;
  count: number;
  addIcon: React.ReactNode;
  onAdd: () => void;
  onEnable: () => void;
  onDisable: () => void;
  onDelete: () => void;
  selectedCount: number;
  busy: boolean;
  children: React.ReactNode;
}) {
  const hasSelection = props.selectedCount > 0;
  return (
    <section className="flex min-h-0 flex-col">
      <div className="flex items-center justify-between gap-2 px-3 py-2">
        <div className="flex items-center gap-2">
          <h2 className="font-heading font-medium text-sm">{props.title}</h2>
          <span className="rounded-full bg-muted px-1.5 py-0.5 text-muted-foreground text-xs">
            {props.count}
          </span>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={props.onAdd}
          disabled={props.busy}
          aria-label={`Add to ${props.title}`}
        >
          {props.addIcon}
        </Button>
      </div>

      <div className="flex items-center gap-1 border-y bg-muted/40 px-3 py-1.5">
        <Button
          variant="ghost"
          size="icon-xs"
          disabled={!hasSelection || props.busy}
          onClick={props.onEnable}
          aria-label="Enable selected"
          title="Включить выбранные"
        >
          <Check />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          disabled={!hasSelection || props.busy}
          onClick={props.onDisable}
          aria-label="Disable selected"
          title="Выключить выбранные"
        >
          <X />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          disabled={!hasSelection || props.busy}
          onClick={props.onDelete}
          aria-label="Delete selected"
          title="Удалить выбранные из хранилища"
        >
          <Trash2 />
        </Button>
        {hasSelection && (
          <span className="ml-1 text-muted-foreground text-xs">{props.selectedCount} выбрано</span>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-2">{props.children}</div>
    </section>
  );
}

function ProjectGroup(props: {
  project: ProjectInfo;
  selected: Set<string>;
  busy: boolean;
  onToggleSelect: (ref: SkillRef) => void;
  onToggleEnabled: (ref: SkillRef, enabled: boolean) => void;
  onRemove: () => void;
}) {
  const { project } = props;
  return (
    <div className="mb-3">
      <div className="flex items-center justify-between gap-2 px-1 py-1">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 font-medium text-sm">
            <span className="truncate">{project.name}</span>
            {!project.exists && (
              <span className="text-destructive" title="Путь проекта не найден">
                <AlertTriangle className="size-3.5" />
              </span>
            )}
          </div>
          <p className="truncate text-muted-foreground text-xs">
            {project.path || "путь не задан"}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={props.onRemove}
          disabled={props.busy}
          aria-label="Remove project"
          title="Убрать проект"
        >
          <X />
        </Button>
      </div>
      {project.skills.map((skill) => {
        const ref: SkillRef = {
          scope: "project",
          project: project.name,
          dirName: skill.dirName,
        };
        return (
          <SkillRow
            key={skill.dirName}
            skill={skill}
            selected={props.selected.has(skillKey(ref))}
            busy={props.busy || !project.exists}
            onToggleSelect={() => props.onToggleSelect(ref)}
            onToggleEnabled={(v) => props.onToggleEnabled(ref, v)}
          />
        );
      })}
      {project.skills.length === 0 && <EmptyHint text="В проекте нет скиллов." />}
    </div>
  );
}

function SkillRow(props: {
  skill: SkillMeta;
  selected: boolean;
  busy: boolean;
  onToggleSelect: () => void;
  onToggleEnabled: (enabled: boolean) => void;
}) {
  const { skill } = props;
  return (
    <div className="flex items-center gap-2.5 rounded-lg px-2 py-1.5 hover:bg-muted/50">
      <Switch
        checked={skill.enabled}
        disabled={props.busy}
        onCheckedChange={props.onToggleEnabled}
        aria-label={`Toggle ${skill.name}`}
      />
      <Checkbox
        checked={props.selected}
        onCheckedChange={props.onToggleSelect}
        aria-label={`Select ${skill.name}`}
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="truncate text-sm">{skill.name}</span>
          {!skill.valid && (
            <span className="text-destructive" title={skill.error ?? "Некорректный SKILL.md"}>
              <AlertTriangle className="size-3.5" />
            </span>
          )}
        </div>
        {skill.description && (
          <p className="truncate text-muted-foreground text-xs" title={skill.description}>
            {skill.description}
          </p>
        )}
      </div>
    </div>
  );
}

function EmptyHint({ text }: { text: string }) {
  return <p className="px-2 py-6 text-center text-muted-foreground text-xs">{text}</p>;
}
