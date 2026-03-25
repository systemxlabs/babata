import type { PropsWithChildren, ReactNode } from 'react';

interface PanelProps extends PropsWithChildren {
  eyebrow?: string;
  title?: string;
  meta?: ReactNode;
  className?: string;
}

export function Panel({ eyebrow, title, meta, className, children }: PanelProps) {
  const panelClassName = className ? `panel ${className}` : 'panel';

  return (
    <section className={panelClassName}>
      {(eyebrow || title || meta) && (
        <header className="panel__header">
          <div className="panel__heading">
            {eyebrow ? <p className="panel__eyebrow">{eyebrow}</p> : null}
            {title ? <h2 className="panel__title">{title}</h2> : null}
          </div>
          {meta ? <div className="panel__meta">{meta}</div> : null}
        </header>
      )}
      <div className="panel__body">{children}</div>
    </section>
  );
}
