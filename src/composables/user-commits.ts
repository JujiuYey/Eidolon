// Git 功能已移除
export function useCommits() {
  const fetchLoading = ref(false);

  const fetchGitLog = async () => {
    return [];
  };

  return {
    fetchLoading,
    fetchGitLog,
  };
}
