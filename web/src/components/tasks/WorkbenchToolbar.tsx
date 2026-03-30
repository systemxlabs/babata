interface WorkbenchToolbarProps {
  onViewChange: (view: 'root' | 'timeline') => void;
  view: 'root' | 'timeline';
}

export function WorkbenchToolbar({
  onViewChange,
  view,
}: WorkbenchToolbarProps) {
  return (
    <div className="workbench-toolbar">
      <div className="workbench-toggle" role="group" aria-label="Task list mode">
        <button
          aria-pressed={view === 'root'}
          className={
            view === 'root'
              ? 'workbench-toggle__button workbench-toggle__button--active'
              : 'workbench-toggle__button'
          }
          onClick={() => onViewChange('root')}
          type="button"
        >
          Root tasks
        </button>
        <button
          aria-pressed={view === 'timeline'}
          className={
            view === 'timeline'
              ? 'workbench-toggle__button workbench-toggle__button--active'
              : 'workbench-toggle__button'
          }
          onClick={() => onViewChange('timeline')}
          type="button"
        >
          Timeline
        </button>
      </div>
    </div>
  );
}
