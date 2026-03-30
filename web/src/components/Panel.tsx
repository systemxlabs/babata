import { createElement, type PropsWithChildren, type ReactNode } from 'react';

type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;

interface PanelProps extends PropsWithChildren {
  eyebrow?: string;
  title?: string;
  titleLevel?: HeadingLevel;
  meta?: ReactNode;
  className?: string;
  bodyClassName?: string;
}

export function Panel({
  eyebrow,
  title,
  titleLevel = 2,
  meta,
  className,
  bodyClassName,
  children,
}: PanelProps) {
  const panelClassName = ['panel', className].filter(Boolean).join(' ');
  const panelBodyClassName = ['panel__body', bodyClassName].filter(Boolean).join(' ');

  return (
    <section className={panelClassName}>
      {(eyebrow || title || meta) && (
        <header className="panel__header">
          <div className="panel__heading">
            {eyebrow ? <p className="panel__eyebrow">{eyebrow}</p> : null}
            {title ? createElement(`h${titleLevel}`, { className: 'panel__title' }, title) : null}
          </div>
          {meta ? <div className="panel__meta">{meta}</div> : null}
        </header>
      )}
      <div className={panelBodyClassName}>{children}</div>
    </section>
  );
}
