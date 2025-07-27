# GeekTools Plugin Marketplace Frontend

一个精美的插件市场前端演示，基于 Tailwind CSS 构建，兼容后端 API 接口设计。

## 特性

- 🎨 **精美设计**: 参考 Claude 官网风格，使用圆角图标和渐变效果
- 📱 **响应式布局**: 完全适配桌面端和移动端
- 🔍 **智能搜索**: 支持关键词搜索和分类筛选
- 📊 **数据统计**: 插件统计信息展示
- 🔄 **实时更新**: 动态加载和分页功能
- 📤 **文件上传**: 拖拽上传插件文件
- 🎯 **用户体验**: 流畅的动画和交互效果

## 文件结构

```
plugin_server/
├── index.html          # 主页面
├── app.js              # JavaScript 应用逻辑
└── README.md           # 项目说明
```

## 技术栈

- **HTML5**: 语义化标记
- **Tailwind CSS**: 响应式 CSS 框架
- **Vanilla JavaScript**: 原生 JavaScript，无依赖
- **Font Awesome**: 图标库

## API 接口兼容

前端已按照 `../docs/plugin-marketplace-server.md` 中的 API 设计进行开发，包括：

### 主要接口
- `GET /api/v1/plugins` - 获取插件列表
- `GET /api/v1/plugins/{id}` - 获取插件详情
- `POST /api/v1/plugins` - 上传插件
- `GET /api/v1/plugins/{id}/download` - 下载插件

### 查询参数
- `page` - 页码
- `limit` - 每页数量
- `search` - 搜索关键词
- `tag` - 标签过滤
- `sort` - 排序方式

## 功能特性

### 1. 插件浏览
- 网格布局展示插件卡片
- 显示插件名称、描述、作者、下载量、评分
- 支持分页浏览

### 2. 搜索和筛选
- 实时搜索插件名称和描述
- 按分类筛选
- 多种排序方式（下载量、评分、更新时间、名称）

### 3. 插件详情
- 模态框展示详细信息
- 版本历史和更新日志
- 包含的脚本列表
- 下载和统计信息

### 4. 插件上传
- 支持拖拽上传 .tar.gz 文件
- 文件格式和大小验证
- 上传进度提示

### 5. 响应式设计
- 移动端友好
- 自适应网格布局
- 触摸友好的交互

## 使用方法

1. **本地预览**
   ```bash
   # 在 plugin_server 目录下启动本地服务器
   python -m http.server 8080
   # 或使用 Node.js
   npx serve .
   ```

2. **访问页面**
   打开浏览器访问 `http://localhost:8080`

## 配置说明

### API 基础 URL
在 `app.js` 中修改 `baseURL` 配置：
```javascript
this.baseURL = 'https://api.geektools.dev/v1';
```

### 模拟数据
当前使用模拟数据进行演示，在实际部署时需要：
1. 替换 `getMockPlugins()` 方法为真实 API 调用
2. 实现真实的文件上传逻辑
3. 添加用户认证功能

## 样式定制

### 主题颜色
在 Tailwind 配置中定义了自定义颜色：
```javascript
colors: {
    'claude-orange': '#FF8C47',
    'claude-bg': '#F9F9F8',
    'claude-text': '#2F2F2F',
    'claude-light': '#FEFEFE',
}
```

### 字体
使用系统字体栈确保最佳显示效果：
```javascript
fontFamily: {
    'claude': ['system-ui', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
}
```

## 浏览器支持

- Chrome 60+
- Firefox 60+
- Safari 12+
- Edge 79+

## 性能优化

- 使用 CDN 加载 Tailwind CSS
- 图片懒加载
- 分页减少数据量
- 防抖搜索减少请求

## 后续开发建议

1. **用户系统**: 添加用户注册/登录功能
2. **评论系统**: 插件评论和评分功能
3. **收藏功能**: 用户收藏插件
4. **插件分析**: 下载统计和使用分析
5. **版本管理**: 更完善的版本控制
6. **安全性**: 添加 CSRF 保护和输入验证

## License

MIT License