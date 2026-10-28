use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection};

use crate::{error::ApiResult, services::markdown};

const TAGS: &[(&str, &str)] = &[
    ("技术", "technology"),
    ("写作", "writing"),
    ("阅读", "reading"),
    ("生活", "life"),
    ("架构", "architecture"),
    ("本地优先", "local-first"),
    ("Node.js", "nodejs"),
    ("工作流", "workflow"),
];

struct SeedPost {
    title: &'static str,
    slug: &'static str,
    excerpt: &'static str,
    markdown: &'static str,
    status: &'static str,
    days_ago: Option<i64>,
    cover: &'static str,
    tags: &'static [&'static str],
}

const POSTS: &[SeedPost] = &[
    SeedPost {
        title: "把本地优先写进博客的骨架",
        slug: "local-first-blog-architecture",
        excerpt: "从一份可备份的 SQLite 数据库开始，重新思考博客的拥有权、边界与部署方式。",
        markdown: r#"## 从拥有数据开始

本地优先不是拒绝网络，而是让网络断开时，创作仍然成立。这个博客原型把文章、标签和附件索引放在同一份 SQLite 数据库中，正文则同时保留 Markdown 与净化后的 HTML。

> 工具应该延长注意力的寿命，而不是把注意力锁进平台。

### 三个清晰的边界

- **内容层**：Markdown 是唯一便于迁移的源文本。
- **服务层**：渲染、校验和发布规则不依赖 HTTP 框架。
- **存储层**：本地文件与 SQLite 可以在未来替换成对象存储和 D1。

```text
Markdown -> 渲染与净化 -> SQLite -> 公共 API
                     \-> 文件存储 -> 图片 URL
```

当每个边界都能单独测试，替换实现就不再是一场重写。定期复制数据库和附件目录，便得到一份完整、可离线阅读的备份。"#,
        status: "published",
        days_ago: Some(7),
        cover: "/covers/garden.svg",
        tags: &["architecture", "local-first", "technology"],
    },
    SeedPost {
        title: "Node.js SQLite：给小工具一个可靠底座",
        slug: "small-tools-reliable-sqlite",
        excerpt: "事务、唯一约束和明确索引，比追逐复杂基础设施更能保护一个小型内容系统。",
        markdown: r#"## 小，不等于随意

个人博客通常不需要一套数据库集群，但需要和大型系统一样认真对待约束。文章别名必须唯一，标签关系依赖外键，上传内容则用摘要避免重复。

### 先写不变量

1. 发布文章必须有发布时间。
2. 更新文章必须携带版本号。
3. 删除默认是归档，而不是抹掉历史。

```sql
UPDATE posts
SET title = ?, version = version + 1
WHERE id = ? AND version = ?;
```

> 可靠性往往来自一句可检查的约束，而不是一张宏大的架构图。

SQLite 的事务足够覆盖这些规则。配合 WAL 模式和定期备份，小工具也能拥有稳定、可解释的持久层。"#,
        status: "published",
        days_ago: Some(32),
        cover: "/covers/sqlite.svg",
        tags: &["technology", "nodejs", "architecture"],
    },
    SeedPost {
        title: "写作不是输出，而是整理注意力",
        slug: "writing-organizes-attention",
        excerpt: "把写作从产能指标里解救出来：先收集问题，再安排证据，最后才润色句子。",
        markdown: r#"## 草稿是一份注意力地图

面对空白页面时，我常常误以为自己缺少表达。更多时候，真正缺少的是对问题的排序：哪件事最重要，哪段经验能够作证，哪个细节应该舍弃。

### 我的三段式草稿

- 用一句话写下读者读完后的变化。
- 只列证据，不急着写过渡句。
- 删除所有不能支持中心变化的段落。

> 好文章不是知道得多，而是终于知道什么可以不说。

```text
问题 -> 证据 -> 取舍 -> 句子
```

把润色放在最后，能减少漂亮句子对判断的绑架。写作因此不再只是输出，而成为整理经验、校正注意力的过程。"#,
        status: "published",
        days_ago: Some(68),
        cover: "/covers/letters.svg",
        tags: &["writing", "workflow"],
    },
    SeedPost {
        title: "建立可重读的阅读笔记",
        slug: "reusable-reading-notes",
        excerpt: "少摘抄一点，多留下问题、判断与回访日期，让阅读笔记真正参与下一次思考。",
        markdown: r#"## 笔记要经得起重逢

摘抄保存了作者的话，却未必保存自己的理解。我现在的每张阅读卡片只回答三个问题：作者试图改变什么，我哪里不同意，它会改变哪一个具体行动。

### 一张卡片的结构

- **主张**：不用原文，用自己的话复述。
- **张力**：记录疑惑或反例。
- **连接**：链接到一本书、一篇文章或一次实践。

> 重读不是再次经过文字，而是带着新的自己与旧判断辩论。

```yaml
source: 书名或文章
claim: 一句话主张
question: 尚未解决的问题
revisit: 2026-11-01
```

每隔一个月随机回访几张卡片。无法被再次使用的笔记可以合并，已经改变行动的笔记则值得扩写成文章。"#,
        status: "published",
        days_ago: Some(112),
        cover: "/covers/rss.svg",
        tags: &["reading", "writing", "workflow"],
    },
    SeedPost {
        title: "把一周过成可以回顾的文本",
        slug: "a-week-worth-reviewing",
        excerpt: "一份不追求自律表演的周记模板，用来记录能量、关系和真正完成的小事。",
        markdown: r#"## 留下生活的刻度

周记不该只是任务清单的副本。完成的任务说明时间去了哪里，而疲惫、惊喜和谈话说明生活为何如此流动。

### 周日晚上的十五分钟

1. 写下三件值得记住的小事。
2. 标记最消耗和最恢复能量的时刻。
3. 给下周留下一个足够轻的意图。

> 回顾不是给生活打分，而是让模糊的日子重新拥有轮廓。

```text
本周关键词：缓慢
想继续：午后散步
想减少：睡前无目的刷屏
```

连续记录几周后，重复出现的词比偶尔高涨的情绪更诚实。它们帮助我调整日程，也提醒我不要只在高效时才认可自己。"#,
        status: "published",
        days_ago: Some(166),
        cover: "/covers/night.svg",
        tags: &["life", "writing"],
    },
    SeedPost {
        title: "从草稿到发布：一套轻量编辑流程",
        slug: "lightweight-editorial-workflow",
        excerpt: "用状态、版本和一次冷静的重读，建立既不妨碍灵感也能减少事故的发布流程。",
        markdown: r#"## 流程应该保护作品，而不是制造手续

个人写作不需要复杂审批，但仍需要几个明确的停顿。草稿用来容纳混乱，发布前检查用来保护读者，归档则保留作品曾经存在的历史。

### 发布前的四问

- 标题承诺的内容是否真正交付？
- 代码和链接是否仍然可用？
- 引用是否准确，图片是否有替代文本？
- 删除哪一段，文章反而更完整？

> 编辑不是把文字变得规整，而是让读者少遇到一点不必要的阻力。

```text
草稿 -> 冷却一晚 -> 重读 -> 发布 -> 必要时归档
```

版本号能防止两个编辑窗口相互覆盖，归档则让删除不再冲动。少量规则足以支撑长期写作。"#,
        status: "published",
        days_ago: Some(225),
        cover: "/covers/toolbox.svg",
        tags: &["writing", "workflow", "technology"],
    },
    SeedPost {
        title: "正在试验：用版本号处理编辑冲突",
        slug: "optimistic-locking-notes",
        excerpt: "一篇仍在验证中的草稿，记录乐观锁如何提示覆盖风险。",
        markdown: r#"## 尚未完成的实验

两个编辑窗口同时打开时，最后保存的人不应无声覆盖先前的修改。我正在给每次更新附加版本号，并把冲突留给编辑者决定。

- 先保留双方文本。
- 再提示版本已经变化。
- 绝不自动丢弃新内容。

> 草稿的价值，在于允许结论暂时保持开放。

```text
读取 version = 3
更新 where version = 3
影响行数为 0 -> 返回冲突
```

接下来还需要测试自动保存与手动保存交错的情况。"#,
        status: "draft",
        days_ago: None,
        cover: "/covers/edge.svg",
        tags: &["technology", "architecture"],
    },
    SeedPost {
        title: "归档：我的第一套博客约定",
        slug: "archived-first-blog-conventions",
        excerpt: "早期博客约定的历史快照，保留用于观察写作系统的变化。",
        markdown: r#"## 为什么保留旧约定

这篇文章记录第一版博客的分类方式，现已由标签与归档取代。它不再指导当前实践，却能解释一些旧链接和旧文章的结构。

- 每篇文章只能属于一个分类。
- 封面必须放在固定目录。
- 删除文章会同时删除评论。

> 归档不是继续推荐，而是诚实保存来路。

```text
category -> tags
delete   -> archive
```

新系统允许多个标签，并把默认删除改为归档。旧约定留在这里，作为设计变化的对照。"#,
        status: "archived",
        days_ago: Some(310),
        cover: "/covers/default.svg",
        tags: &["life", "architecture"],
    },
];

pub fn seed(connection: &mut Connection) -> ApiResult<()> {
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM posts", [], |row| row.get(0))?;
    let now = Utc::now();
    let timestamp = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let transaction = connection.transaction()?;

    for (name, slug) in TAGS {
        transaction.execute(
            "INSERT OR IGNORE INTO tags (name, slug) VALUES (?1, ?2)",
            params![name, slug],
        )?;
    }

    for (key, value) in [
        ("site_title", "\"栖迟\""),
        ("site_description", "\"记录技术、阅读与缓慢生活\""),
        ("site_language", "\"zh-CN\""),
    ] {
        transaction.execute(
            "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, timestamp],
        )?;
    }

    if count == 0 {
        for post in POSTS {
            let published_at = post.days_ago.map(|days| {
                (now - Duration::days(days)).to_rfc3339_opts(SecondsFormat::Millis, true)
            });
            let content_html = markdown::render(post.markdown)?;
            transaction.execute(
                r#"
                INSERT INTO posts (
                    title, slug, excerpt, content_markdown, content_html, status,
                    published_at, created_at, updated_at, version, cover_image
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10)
                "#,
                params![
                    post.title,
                    post.slug,
                    post.excerpt,
                    post.markdown,
                    content_html,
                    post.status,
                    published_at,
                    timestamp,
                    timestamp,
                    post.cover,
                ],
            )?;
            let post_id = transaction.last_insert_rowid();
            for slug in post.tags {
                transaction.execute(
                    r#"
                    INSERT INTO post_tags (post_id, tag_id)
                    SELECT ?1, id FROM tags WHERE slug = ?2
                    "#,
                    params![post_id, slug],
                )?;
            }
        }
    }

    transaction.commit()?;
    Ok(())
}
