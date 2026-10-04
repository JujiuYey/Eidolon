import type {
  FetchCommitsRequest,
  PolishWeeklyReportInput,
  PolishWeeklyReportResult,
  RepoCommitLog,
  WeeklyReportEntry,
  WeeklyReportRepo,
} from '@/types/weekly-report';
import { invoke } from '@tauri-apps/api/core';

export async function listWeeklyReportRepos(): Promise<WeeklyReportRepo[]> {
  return invoke<WeeklyReportRepo[]>('list_weekly_report_repos');
}

export async function addWeeklyReportRepo(path: string): Promise<WeeklyReportRepo> {
  return invoke<WeeklyReportRepo>('add_weekly_report_repo', { path });
}

export async function removeWeeklyReportRepo(repoId: string): Promise<string> {
  return invoke<string>('remove_weekly_report_repo', { repoId });
}

export async function fetchWeeklyReportCommits(
  request: FetchCommitsRequest,
): Promise<RepoCommitLog[]> {
  return invoke<RepoCommitLog[]>('fetch_weekly_report_commits', { request });
}

export async function listWeeklyReports(): Promise<WeeklyReportEntry[]> {
  return invoke<WeeklyReportEntry[]>('list_weekly_reports');
}

export async function saveWeeklyReport(report: WeeklyReportEntry): Promise<WeeklyReportEntry> {
  return invoke<WeeklyReportEntry>('save_weekly_report', { report });
}

export async function deleteWeeklyReport(reportId: string): Promise<string> {
  return invoke<string>('delete_weekly_report', { reportId });
}

export async function exportWeeklyReport(path: string, content: string): Promise<string> {
  return invoke<string>('export_weekly_report', { path, content });
}

export async function polishWeeklyReport(
  input: PolishWeeklyReportInput,
): Promise<PolishWeeklyReportResult> {
  return invoke<PolishWeeklyReportResult>('polish_weekly_report', { input });
}
