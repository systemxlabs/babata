import './TaskPagination.css';

interface TaskPaginationProps {
  currentPage: number;
  pageSize: number;
  total: number;
  onPageChange: (page: number) => void;
}

export function TaskPagination({ currentPage, pageSize, total, onPageChange }: TaskPaginationProps) {
  const totalPages = Math.ceil(total / pageSize);
  
  if (totalPages <= 1) return null;

  const getPageNumbers = () => {
    const pages: (number | string)[] = [];
    const maxVisiblePages = 5;

    if (totalPages <= maxVisiblePages) {
      for (let i = 1; i <= totalPages; i++) {
        pages.push(i);
      }
    } else {
      // 总是显示第一页
      pages.push(1);

      if (currentPage > 3) {
        pages.push('...');
      }

      // 显示当前页附近的页面
      const startPage = Math.max(2, currentPage - 1);
      const endPage = Math.min(totalPages - 1, currentPage + 1);

      for (let i = startPage; i <= endPage; i++) {
        if (!pages.includes(i)) {
          pages.push(i);
        }
      }

      if (currentPage < totalPages - 2) {
        pages.push('...');
      }

      // 总是显示最后一页
      if (!pages.includes(totalPages)) {
        pages.push(totalPages);
      }
    }

    return pages;
  };

  const pages = getPageNumbers();

  return (
    <div className="task-pagination">
      <div className="pagination-info">
        共 <span className="total-count">{total}</span> 条记录，
        每页 <span className="page-size">{pageSize}</span> 条
      </div>

      <div className="pagination-controls">
        <button
          className="pagination-btn"
          onClick={() => onPageChange(currentPage - 1)}
          disabled={currentPage <= 1}
          title="上一页"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="m15 18-6-6 6-6" />
          </svg>
        </button>

        {pages.map((page, index) => (
          page === '...' ? (
            <span key={`ellipsis-${index}`} className="pagination-ellipsis">...</span>
          ) : (
            <button
              key={page}
              className={`pagination-btn ${currentPage === page ? 'active' : ''}`}
              onClick={() => onPageChange(page as number)}
            >
              {page}
            </button>
          )
        ))}

        <button
          className="pagination-btn"
          onClick={() => onPageChange(currentPage + 1)}
          disabled={currentPage >= totalPages}
          title="下一页"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="m9 18 6-6-6-6" />
          </svg>
        </button>
      </div>
    </div>
  );
}
