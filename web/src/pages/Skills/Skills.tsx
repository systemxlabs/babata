import { useState, useEffect } from 'react';
import { api } from '../../api';
import type { Skill } from '../../types';
import './Skills.css';

export function Skills() {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // 获取技能列表
  const fetchSkills = async () => {
    try {
      setLoading(true);
      const response = await api.getSkills();
      setSkills(response.skills);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取技能列表失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSkills();
  }, []);

  // 删除技能
  const handleDelete = async (skill: Skill) => {
    const confirmed = window.confirm(
      `确定要删除技能 "${skill.name}" 吗？\n\n此操作不可撤销。`
    );

    if (!confirmed) return;

    try {
      // 调用删除 API
      const response = await fetch(`/api/skills/${encodeURIComponent(skill.name)}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new Error(`删除失败: ${response.status} ${response.statusText}`);
      }

      // 删除成功后刷新列表
      await fetchSkills();
    } catch (err) {
      setError(err instanceof Error ? err.message : '删除技能失败');
    }
  };

  if (loading) {
    return (
      <div className="skills-page">
        <h1>🛠️ 技能管理</h1>
        <div className="loading-state">
          <div className="loading-spinner"></div>
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  if (error && skills.length === 0) {
    return (
      <div className="skills-page">
        <h1>🛠️ 技能管理</h1>
        <div className="error-state">
          <p>❌ {error}</p>
          <button onClick={fetchSkills}>重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="skills-page">
      <div className="skills-header">
        <h1>🛠️ 技能管理</h1>
        <span className="skills-count">共 {skills.length} 个技能</span>
      </div>

      {/* 错误提示（非空状态时显示）*/}
      {error && (
        <div className="skills-error-banner">
          <span>❌ {error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      {skills.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">🛠️</div>
          <p>暂无配置的技能</p>
        </div>
      ) : (
        <div className="skills-grid">
          {skills.map((skill) => (
            <div key={skill.name} className="skill-card">
              <div className="skill-card-content">
                <div className="skill-icon">🛠️</div>
                <div className="skill-info">
                  <h3 className="skill-name">{skill.name}</h3>
                  <p className="skill-description">
                    {skill.description || '暂无描述'}
                  </p>
                </div>
              </div>
              <button
                className="skill-delete-btn"
                onClick={() => handleDelete(skill)}
                title="删除技能"
              >
                🗑️
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
