// GeekTools Plugin Marketplace Frontend
class PluginMarketplace {
    constructor() {
        this.baseURL = '/api/v1';
        this.currentPage = 1;
        this.pageSize = 20;
        this.currentQuery = '';
        this.currentCategory = '';
        this.currentSort = 'downloads';
        this.plugins = [];
        this.totalPages = 0;
        this.authToken = localStorage.getItem('auth_token');
        this.currentUser = JSON.parse(localStorage.getItem('current_user') || 'null');
        
        this.init();
    }

    init() {
        this.bindEvents();
        this.initAuth();
        this.loadStats();
        this.loadPlugins();
        this.setupFileUpload();
    }

    initAuth() {
        this.updateAuthUI();
    }

    updateAuthUI() {
        const loggedOut = document.getElementById('loggedOut');
        const loggedIn = document.getElementById('loggedIn');
        const userEmail = document.getElementById('userEmail');
        const userRole = document.getElementById('userRole');
        const adminPanelLink = document.getElementById('adminPanelLink');

        if (this.authToken && this.currentUser) {
            loggedOut.classList.add('hidden');
            loggedIn.classList.remove('hidden');
            userEmail.textContent = this.currentUser.email;
            
            // Show admin-specific UI elements
            if (this.currentUser.role === 'admin') {
                userRole.classList.remove('hidden');
                adminPanelLink.classList.remove('hidden');
            } else {
                userRole.classList.add('hidden');
                adminPanelLink.classList.add('hidden');
            }
        } else {
            loggedOut.classList.remove('hidden');
            loggedIn.classList.add('hidden');
            userRole.classList.add('hidden');
            adminPanelLink.classList.add('hidden');
        }
    }

    showLoginModal() {
        document.getElementById('loginModal').classList.remove('hidden');
        document.body.style.overflow = 'hidden';
        this.resetLoginForm();
    }

    hideLoginModal() {
        document.getElementById('loginModal').classList.add('hidden');
        document.body.style.overflow = 'auto';
        this.resetLoginForm();
    }

    resetLoginForm() {
        document.getElementById('emailInput').value = '';
        document.getElementById('codeInput').value = '';
        document.getElementById('sendCodeStep').classList.remove('hidden');
        document.getElementById('verifyCodeStep').classList.add('hidden');
        document.getElementById('codeDisplay').classList.add('hidden');
    }

    async sendVerificationCode() {
        const emailInput = document.getElementById('emailInput');
        const email = emailInput.value.trim();

        if (!email) {
            this.showError('请输入邮箱地址');
            return;
        }

        const sendCodeBtn = document.getElementById('sendCodeBtn');
        const originalText = sendCodeBtn.textContent;
        sendCodeBtn.disabled = true;
        sendCodeBtn.textContent = '发送中...';

        try {
            const response = await fetch(`${this.baseURL}/auth/send-code`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ email })
            });

            const data = await response.json();

            if (response.ok && data.success) {
                // Show verification code step
                document.getElementById('sendCodeStep').classList.add('hidden');
                document.getElementById('verifyCodeStep').classList.remove('hidden');

                // Display code if provided (when SMTP not configured)
                if (data.data.code) {
                    document.getElementById('displayedCode').textContent = data.data.code;
                    document.getElementById('codeDisplay').classList.remove('hidden');
                }

                this.showSuccess(data.data.message || '验证码已发送');
            } else {
                throw new Error(data.error || '发送验证码失败');
            }
        } catch (error) {
            console.error('Send verification code failed:', error);
            this.showError(`发送验证码失败: ${error.message}`);
        } finally {
            sendCodeBtn.disabled = false;
            sendCodeBtn.textContent = originalText;
        }
    }

    async verifyCodeAndLogin() {
        const emailInput = document.getElementById('emailInput');
        const codeInput = document.getElementById('codeInput');
        const email = emailInput.value.trim();
        const code = codeInput.value.trim();

        if (!email || !code) {
            this.showError('请输入邮箱和验证码');
            return;
        }

        if (code.length !== 6) {
            this.showError('验证码应为6位数字');
            return;
        }

        const verifyBtn = document.getElementById('verifyCodeBtn');
        const verifyText = document.getElementById('verifyText');
        const loginSpinner = document.getElementById('loginSpinner');

        verifyBtn.disabled = true;
        verifyText.textContent = '验证中...';
        loginSpinner.classList.remove('hidden');

        try {
            const response = await fetch(`${this.baseURL}/auth/verify-code`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ email, code })
            });

            const data = await response.json();

            if (response.ok && data.success) {
                // Store auth data
                this.authToken = data.data.token;
                this.currentUser = data.data.user;
                localStorage.setItem('auth_token', this.authToken);
                localStorage.setItem('current_user', JSON.stringify(this.currentUser));

                // Update UI
                this.updateAuthUI();
                this.hideLoginModal();
                this.showSuccess(data.message || '登录成功！');
            } else {
                throw new Error(data.error || '验证失败');
            }
        } catch (error) {
            console.error('Verify code failed:', error);
            this.showError(`验证失败: ${error.message}`);
        } finally {
            verifyBtn.disabled = false;
            verifyText.textContent = '登录';
            loginSpinner.classList.add('hidden');
        }
    }

    backToEmailStep() {
        document.getElementById('sendCodeStep').classList.remove('hidden');
        document.getElementById('verifyCodeStep').classList.add('hidden');
        document.getElementById('codeDisplay').classList.add('hidden');
        document.getElementById('codeInput').value = '';
    }

    logout() {
        this.authToken = null;
        this.currentUser = null;
        localStorage.removeItem('auth_token');
        localStorage.removeItem('current_user');
        this.updateAuthUI();
        this.showSuccess('已退出登录');
    }

    bindEvents() {
        // Search
        const searchInput = document.getElementById('searchInput');
        let searchTimeout;
        searchInput.addEventListener('input', (e) => {
            clearTimeout(searchTimeout);
            searchTimeout = setTimeout(() => {
                this.currentQuery = e.target.value;
                this.currentPage = 1;
                this.loadPlugins();
            }, 300);
        });

        // Filters
        document.getElementById('categoryFilter').addEventListener('change', (e) => {
            this.currentCategory = e.target.value;
            this.currentPage = 1;
            this.loadPlugins();
        });

        document.getElementById('sortFilter').addEventListener('change', (e) => {
            this.currentSort = e.target.value;
            this.currentPage = 1;
            this.loadPlugins();
        });

        // Pagination
        document.getElementById('prevPage').addEventListener('click', () => {
            if (this.currentPage > 1) {
                this.currentPage--;
                this.loadPlugins();
            }
        });

        document.getElementById('nextPage').addEventListener('click', () => {
            if (this.currentPage < this.totalPages) {
                this.currentPage++;
                this.loadPlugins();
            }
        });

        // Upload buttons
        document.getElementById('uploadBtn').addEventListener('click', () => {
            if (!this.authToken) {
                this.showError('请先登录后再上传插件');
                return;
            }
            this.showUploadModal();
        });

        document.getElementById('uploadBtnLoggedIn').addEventListener('click', () => {
            this.showUploadModal();
        });

        document.getElementById('closeUploadModal').addEventListener('click', () => {
            this.hideUploadModal();
        });

        document.getElementById('cancelUpload').addEventListener('click', () => {
            this.hideUploadModal();
        });

        // Close modal when clicking outside
        document.getElementById('pluginModal').addEventListener('click', (e) => {
            if (e.target.id === 'pluginModal') {
                this.hidePluginModal();
            }
        });

        document.getElementById('uploadModal').addEventListener('click', (e) => {
            if (e.target.id === 'uploadModal') {
                this.hideUploadModal();
            }
        });

        // Upload form
        document.getElementById('uploadForm').addEventListener('submit', (e) => {
            e.preventDefault();
            this.uploadPlugin();
        });

        // Auth events
        document.getElementById('loginBtn').addEventListener('click', () => {
            this.showLoginModal();
        });

        document.getElementById('logoutBtn').addEventListener('click', () => {
            this.logout();
        });

        document.getElementById('closeLoginModal').addEventListener('click', () => {
            this.hideLoginModal();
        });

        document.getElementById('sendCodeBtn').addEventListener('click', () => {
            this.sendVerificationCode();
        });

        document.getElementById('backToEmailBtn').addEventListener('click', () => {
            this.backToEmailStep();
        });

        document.getElementById('loginForm').addEventListener('submit', (e) => {
            e.preventDefault();
            this.verifyCodeAndLogin();
        });
    }

    getAuthToken() {
        // For now, return empty string since we don't have authentication implemented
        // In a real app, this would get the token from localStorage or sessionStorage
        return localStorage.getItem('auth_token') || '';
    }

    async loadStats() {
        try {
            const response = await fetch(`${this.baseURL}/metrics`);
            if (response.ok) {
                const data = await response.json();
                if (data.success) {
                    document.getElementById('totalPlugins').textContent = data.data.total_plugins || '0';
                    document.getElementById('activeDevs').textContent = (data.data.total_users || 0).toLocaleString();
                    document.getElementById('totalDownloads').textContent = (data.data.total_downloads || 0).toLocaleString();
                    document.getElementById('weeklyNew').textContent = (data.data.weekly_new || 0).toLocaleString();
                }
            }
        } catch (error) {
            console.error('Failed to load stats:', error);
            // Fallback to default values
            document.getElementById('totalPlugins').textContent = '0';
            document.getElementById('activeDevs').textContent = '0';
            document.getElementById('totalDownloads').textContent = '0';
            document.getElementById('weeklyNew').textContent = '0';
        }
    }

    async loadPlugins() {
        this.showLoading();
        
        try {
            const params = new URLSearchParams({
                page: this.currentPage.toString(),
                limit: this.pageSize.toString(),
                sort: this.currentSort,
                order: 'desc'
            });
            
            if (this.currentQuery) {
                params.append('search', this.currentQuery);
            }
            if (this.currentCategory) {
                params.append('tag', this.currentCategory);
            }
            
            const response = await fetch(`${this.baseURL}/plugins?${params}`);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            
            const data = await response.json();
            
            if (data.success) {
                this.plugins = data.data.plugins || [];
                this.totalPages = data.data.pagination?.pages || 1;
                
                this.renderPlugins();
                this.renderPagination();
                this.updatePluginCount();
            } else {
                throw new Error(data.error || 'Failed to load plugins');
            }
            
        } catch (error) {
            console.error('Failed to load plugins:', error);
            this.showError('加载插件失败，请稍后重试');
            
            // Fallback to empty state
            this.plugins = [];
            this.totalPages = 1;
            this.renderPlugins();
            this.renderPagination();
            this.updatePluginCount();
        } finally {
            this.hideLoading();
        }
    }

    async getMockPlugins() {
        // Simulate API delay
        await new Promise(resolve => setTimeout(resolve, 500));
        
        const mockData = {
            plugins: [
                {
                    id: 'system_tools',
                    name: '系统工具集',
                    description: '包含常用系统管理和监控工具的插件包，提供系统信息查看、进程管理、磁盘清理等功能',
                    author: 'GeekTools Team',
                    current_version: '1.2.0',
                    downloads: 1250,
                    rating: 4.8,
                    tags: ['system', 'monitoring', 'tools'],
                    created_at: '2024-01-15T10:30:00Z',
                    updated_at: '2024-01-20T14:45:00Z'
                },
                {
                    id: 'dev_utilities',
                    name: '开发者工具',
                    description: '为开发者提供代码格式化、Git 快捷操作、API 测试等实用功能',
                    author: 'DevCommunity',
                    current_version: '2.1.5',
                    downloads: 856,
                    rating: 4.6,
                    tags: ['development', 'git', 'api'],
                    created_at: '2024-01-10T08:20:00Z',
                    updated_at: '2024-01-25T16:30:00Z'
                },
                {
                    id: 'network_analyzer',
                    name: '网络分析器',
                    description: '网络连接测试、端口扫描、DNS 查询等网络诊断工具集合',
                    author: 'NetworkPro',
                    current_version: '1.0.3',
                    downloads: 642,
                    rating: 4.4,
                    tags: ['network', 'diagnostic', 'security'],
                    created_at: '2024-01-08T12:15:00Z',
                    updated_at: '2024-01-22T09:45:00Z'
                },
                {
                    id: 'file_manager',
                    name: '文件管理增强',
                    description: '提供高级文件操作、批量重命名、文件搜索等文件管理功能',
                    author: 'FileExpert',
                    current_version: '3.0.1',
                    downloads: 1120,
                    rating: 4.7,
                    tags: ['utility', 'files', 'productivity'],
                    created_at: '2024-01-05T14:30:00Z',
                    updated_at: '2024-01-24T11:20:00Z'
                },
                {
                    id: 'security_toolkit',
                    name: '安全工具包',
                    description: '系统安全扫描、密码生成、加密解密等安全相关工具',
                    author: 'SecureTeam',
                    current_version: '1.5.2',
                    downloads: 789,
                    rating: 4.9,
                    tags: ['security', 'encryption', 'tools'],
                    created_at: '2024-01-12T16:45:00Z',
                    updated_at: '2024-01-26T13:15:00Z'
                },
                {
                    id: 'media_converter',
                    name: '媒体转换器',
                    description: '支持多种格式的音视频转换、图片处理、格式转换工具',
                    author: 'MediaLab',
                    current_version: '2.3.0',
                    downloads: 567,
                    rating: 4.3,
                    tags: ['media', 'converter', 'utility'],
                    created_at: '2024-01-18T10:00:00Z',
                    updated_at: '2024-01-27T15:30:00Z'
                }
            ],
            pagination: {
                page: this.currentPage,
                limit: this.pageSize,
                total: 156,
                pages: 8
            }
        };

        // Apply filtering and sorting
        let filteredPlugins = mockData.plugins;

        if (this.currentQuery) {
            filteredPlugins = filteredPlugins.filter(plugin => 
                plugin.name.toLowerCase().includes(this.currentQuery.toLowerCase()) ||
                plugin.description.toLowerCase().includes(this.currentQuery.toLowerCase()) ||
                plugin.tags.some(tag => tag.toLowerCase().includes(this.currentQuery.toLowerCase()))
            );
        }

        if (this.currentCategory) {
            filteredPlugins = filteredPlugins.filter(plugin => 
                plugin.tags.includes(this.currentCategory)
            );
        }

        // Sort plugins
        filteredPlugins.sort((a, b) => {
            switch (this.currentSort) {
                case 'downloads':
                    return b.downloads - a.downloads;
                case 'rating':
                    return b.rating - a.rating;
                case 'updated_at':
                    return new Date(b.updated_at) - new Date(a.updated_at);
                case 'name':
                    return a.name.localeCompare(b.name);
                default:
                    return 0;
            }
        });

        return {
            plugins: filteredPlugins,
            pagination: {
                ...mockData.pagination,
                total: filteredPlugins.length,
                pages: Math.ceil(filteredPlugins.length / this.pageSize)
            }
        };
    }

    renderPlugins() {
        const grid = document.getElementById('pluginGrid');
        const emptyState = document.getElementById('emptyState');

        if (this.plugins.length === 0) {
            grid.innerHTML = '';
            emptyState.classList.remove('hidden');
            return;
        }

        emptyState.classList.add('hidden');
        
        grid.innerHTML = this.plugins.map(plugin => this.createPluginCard(plugin)).join('');

        // Add click events to plugin cards
        grid.querySelectorAll('.plugin-card').forEach(card => {
            card.addEventListener('click', () => {
                const pluginId = card.dataset.pluginId;
                this.showPluginDetail(pluginId);
            });
        });
    }

    createPluginCard(plugin) {
        const tags = plugin.tags.slice(0, 3).map(tag => 
            `<span class="px-2 py-1 bg-gray-100 text-gray-600 text-xs rounded-full">${tag}</span>`
        ).join('');

        const stars = this.renderStars(plugin.rating);
        const timeAgo = this.timeAgo(plugin.updated_at);

        return `
            <div class="plugin-card gradient-card rounded-2xl p-6 border border-white/20 hover-scale cursor-pointer" data-plugin-id="${plugin.id}">
                <div class="flex items-start justify-between mb-4">
                    <div class="w-12 h-12 bg-gradient-to-br from-claude-orange to-orange-600 rounded-xl flex items-center justify-center">
                        <i class="fas fa-puzzle-piece text-white text-xl"></i>
                    </div>
                    <div class="flex items-center space-x-1 text-sm text-gray-500">
                        ${stars}
                        <span class="ml-1">${plugin.rating}</span>
                    </div>
                </div>
                
                <h3 class="font-bold text-lg text-claude-text mb-2 line-clamp-1">${plugin.name}</h3>
                <p class="text-gray-600 text-sm mb-4 line-clamp-2">${plugin.description}</p>
                
                <div class="flex flex-wrap gap-1 mb-4">
                    ${tags}
                </div>
                
                <div class="flex items-center justify-between text-sm text-gray-500">
                    <div class="flex items-center space-x-1">
                        <i class="fas fa-user"></i>
                        <span>${plugin.author}</span>
                    </div>
                    <div class="flex items-center space-x-1">
                        <i class="fas fa-download"></i>
                        <span>${plugin.downloads.toLocaleString()}</span>
                    </div>
                </div>
                
                <div class="flex items-center justify-between mt-4 pt-4 border-t border-gray-100">
                    <span class="text-xs text-gray-500">v${plugin.current_version}</span>
                    <span class="text-xs text-gray-500">${timeAgo}</span>
                </div>
            </div>
        `;
    }

    renderStars(rating) {
        const fullStars = Math.floor(rating);
        const hasHalfStar = rating % 1 >= 0.5;
        const emptyStars = 5 - fullStars - (hasHalfStar ? 1 : 0);

        return [
            ...Array(fullStars).fill('<i class="fas fa-star text-yellow-400"></i>'),
            hasHalfStar ? '<i class="fas fa-star-half-alt text-yellow-400"></i>' : '',
            ...Array(emptyStars).fill('<i class="far fa-star text-gray-300"></i>')
        ].join('');
    }

    renderPagination() {
        const paginationSection = document.getElementById('paginationSection');
        const pageNumbers = document.getElementById('pageNumbers');
        const prevBtn = document.getElementById('prevPage');
        const nextBtn = document.getElementById('nextPage');

        if (this.totalPages <= 1) {
            paginationSection.classList.add('hidden');
            return;
        }

        paginationSection.classList.remove('hidden');

        // Update prev/next buttons
        prevBtn.disabled = this.currentPage === 1;
        nextBtn.disabled = this.currentPage === this.totalPages;

        // Generate page numbers
        const pages = this.generatePageNumbers();
        pageNumbers.innerHTML = pages.map(page => {
            if (page === '...') {
                return '<span class="px-3 py-2 text-gray-400">...</span>';
            }
            
            const isActive = page === this.currentPage;
            return `
                <button class="px-3 py-2 ${isActive ? 'bg-claude-orange text-white' : 'bg-white hover:bg-gray-50'} border border-gray-200 rounded-lg text-sm" 
                        onclick="marketplace.goToPage(${page})">${page}</button>
            `;
        }).join('');
    }

    generatePageNumbers() {
        const pages = [];
        const maxVisible = 7;

        if (this.totalPages <= maxVisible) {
            for (let i = 1; i <= this.totalPages; i++) {
                pages.push(i);
            }
        } else {
            if (this.currentPage <= 4) {
                for (let i = 1; i <= 5; i++) pages.push(i);
                pages.push('...');
                pages.push(this.totalPages);
            } else if (this.currentPage >= this.totalPages - 3) {
                pages.push(1);
                pages.push('...');
                for (let i = this.totalPages - 4; i <= this.totalPages; i++) pages.push(i);
            } else {
                pages.push(1);
                pages.push('...');
                for (let i = this.currentPage - 1; i <= this.currentPage + 1; i++) pages.push(i);
                pages.push('...');
                pages.push(this.totalPages);
            }
        }

        return pages;
    }

    goToPage(page) {
        this.currentPage = page;
        this.loadPlugins();
    }

    updatePluginCount() {
        document.getElementById('pluginCount').textContent = this.plugins.length;
    }

    async showPluginDetail(pluginId) {
        try {
            // Make API call to get detailed plugin data
            const response = await fetch(`${this.baseURL}/plugins/${pluginId}`);
            const data = await response.json();
            
            if (response.ok && data.success) {
                // If the API doesn't return versions/scripts, use basic plugin data with empty arrays
                const detailData = {
                    ...data.data,
                    versions: data.data.versions || [],
                    scripts: data.data.scripts || [],
                    total_ratings: data.data.total_ratings || 0
                };
                
                this.renderPluginModal(detailData);
                this.showPluginModal();
            } else {
                throw new Error(data.error || 'Failed to load plugin details');
            }

        } catch (error) {
            console.error('Failed to load plugin details:', error);
            this.showError('加载插件详情失败');
        }
    }

    renderPluginModal(plugin) {
        const modalContent = document.getElementById('modalContent');
        const stars = this.renderStars(plugin.rating);
        const tags = plugin.tags.map(tag => 
            `<span class="px-3 py-1 bg-gray-100 text-gray-700 rounded-full text-sm">${tag}</span>`
        ).join('');

        modalContent.innerHTML = `
            <div class="p-6">
                <div class="flex items-center justify-between mb-6">
                    <div class="flex items-center space-x-4">
                        <div class="w-16 h-16 bg-gradient-to-br from-claude-orange to-orange-600 rounded-2xl flex items-center justify-center">
                            <i class="fas fa-puzzle-piece text-white text-2xl"></i>
                        </div>
                        <div>
                            <h2 class="text-2xl font-bold text-claude-text">${plugin.name}</h2>
                            <p class="text-gray-600">v${plugin.current_version} • ${plugin.author}</p>
                        </div>
                    </div>
                    <button onclick="marketplace.hidePluginModal()" class="text-gray-500 hover:text-gray-700">
                        <i class="fas fa-times text-xl"></i>
                    </button>
                </div>

                <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                    <div class="lg:col-span-2">
                        <div class="mb-6">
                            <h3 class="text-lg font-semibold mb-3">插件描述</h3>
                            <p class="text-gray-700 leading-relaxed">${plugin.description}</p>
                        </div>

                        <div class="mb-6">
                            <h3 class="text-lg font-semibold mb-3">包含脚本</h3>
                            <div class="space-y-3">
                                ${plugin.scripts && plugin.scripts.length > 0 ? 
                                    plugin.scripts.map(script => `
                                        <div class="bg-gray-50 rounded-lg p-4">
                                            <div class="flex items-center justify-between">
                                                <div>
                                                    <h4 class="font-medium text-claude-text">${script.name}</h4>
                                                    <p class="text-sm text-gray-600">${script.description}</p>
                                                    <p class="text-xs text-gray-500 mt-1">文件: ${script.file}</p>
                                                </div>
                                                ${script.executable ? '<span class="px-2 py-1 bg-green-100 text-green-700 text-xs rounded-full">可执行</span>' : ''}
                                            </div>
                                        </div>
                                    `).join('') :
                                    `<div class="text-center py-8 text-gray-500">
                                        <i class="fas fa-code text-3xl mb-2"></i>
                                        <p>暂无脚本信息</p>
                                    </div>`
                                }
                            </div>
                        </div>

                        <div class="mb-6">
                            <h3 class="text-lg font-semibold mb-3">用户评分</h3>
                            <div class="bg-gray-50 rounded-lg p-4">
                                <div class="flex items-center justify-between mb-4">
                                    <div class="flex items-center space-x-2">
                                        ${stars}
                                        <span class="text-lg font-medium">${plugin.rating.toFixed(1)}</span>
                                        <span class="text-gray-500">(${plugin.total_ratings || 0} 评分)</span>
                                    </div>
                                    ${this.authToken ? `
                                        <button onclick="marketplace.showRatingModal('${plugin.id}')" class="px-4 py-2 bg-claude-orange text-white rounded-lg text-sm hover:bg-orange-600 transition-colors">
                                            <i class="fas fa-star mr-1"></i>评分
                                        </button>
                                    ` : `
                                        <button onclick="marketplace.showLoginModal()" class="px-4 py-2 bg-gray-300 text-gray-600 rounded-lg text-sm cursor-not-allowed">
                                            登录后评分
                                        </button>
                                    `}
                                </div>
                                <div id="ratingsList-${plugin.id}" class="space-y-3">
                                    <!-- Ratings will be loaded here -->
                                </div>
                            </div>
                        </div>

                        <div class="mb-6">
                            <h3 class="text-lg font-semibold mb-3">版本历史</h3>
                            <div class="space-y-3">
                                ${plugin.versions && plugin.versions.length > 0 ? 
                                    plugin.versions.map(version => `
                                        <div class="border border-gray-200 rounded-lg p-4">
                                            <div class="flex items-center justify-between mb-2">
                                                <span class="font-medium">v${version.version}</span>
                                                <div class="flex items-center space-x-4 text-sm text-gray-500">
                                                    <span>${this.formatFileSize(version.file_size)}</span>
                                                    <span>${version.downloads} 下载</span>
                                                    <span>${this.timeAgo(version.created_at)}</span>
                                                </div>
                                            </div>
                                            <p class="text-gray-600 text-sm">${version.changelog || '无更新说明'}</p>
                                        </div>
                                    `).join('') : 
                                    `<div class="text-center py-8 text-gray-500">
                                        <i class="fas fa-history text-3xl mb-2"></i>
                                        <p>暂无版本历史记录</p>
                                    </div>`
                                }
                            </div>
                        </div>
                    </div>

                    <div class="lg:col-span-1">
                        <div class="bg-gray-50 rounded-xl p-6 space-y-6">
                            <div>
                                <h3 class="font-semibold mb-3">下载安装</h3>
                                <button onclick="marketplace.downloadPlugin('${plugin.id}')" class="w-full bg-claude-orange text-white py-3 rounded-lg font-medium hover:bg-orange-600 transition-colors">
                                    <i class="fas fa-download mr-2"></i>
                                    下载插件
                                </button>
                            </div>

                            <div>
                                <h3 class="font-semibold mb-3">统计信息</h3>
                                <div class="space-y-2 text-sm">
                                    <div class="flex justify-between">
                                        <span class="text-gray-600">总下载量</span>
                                        <span class="font-medium">${plugin.downloads.toLocaleString()}</span>
                                    </div>
                                    <div class="flex justify-between">
                                        <span class="text-gray-600">评分</span>
                                        <div class="flex items-center space-x-1">
                                            ${stars}
                                            <span class="font-medium ml-1">${plugin.rating}</span>
                                        </div>
                                    </div>
                                    <div class="flex justify-between">
                                        <span class="text-gray-600">最后更新</span>
                                        <span class="font-medium">${this.timeAgo(plugin.updated_at)}</span>
                                    </div>
                                </div>
                            </div>

                            <div>
                                <h3 class="font-semibold mb-3">标签</h3>
                                <div class="flex flex-wrap gap-2">
                                    ${tags}
                                </div>
                            </div>

                            <div>
                                <h3 class="font-semibold mb-3">链接</h3>
                                <div class="space-y-2">
                                    ${plugin.homepage_url ? `
                                        <a href="${plugin.homepage_url}" target="_blank" class="flex items-center text-claude-orange hover:text-orange-600 text-sm">
                                            <i class="fas fa-home mr-2"></i>
                                            项目主页
                                        </a>
                                    ` : ''}
                                    ${plugin.repository_url ? `
                                        <a href="${plugin.repository_url}" target="_blank" class="flex items-center text-claude-orange hover:text-orange-600 text-sm">
                                            <i class="fab fa-github mr-2"></i>
                                            源码仓库
                                        </a>
                                    ` : ''}
                                </div>
                            </div>

                            ${plugin.license ? `
                                <div>
                                    <h3 class="font-semibold mb-3">许可证</h3>
                                    <span class="text-sm text-gray-600">${plugin.license}</span>
                                </div>
                            ` : ''}
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    setupFileUpload() {
        const fileInput = document.getElementById('pluginFile');
        const dropArea = document.getElementById('dropArea');
        const fileInfo = document.getElementById('fileInfo');
        const fileName = document.getElementById('fileName');
        const fileSize = document.getElementById('fileSize');
        const submitBtn = document.getElementById('submitUpload');

        // Click to select file
        dropArea.addEventListener('click', () => {
            fileInput.click();
        });

        // File input change
        fileInput.addEventListener('change', (e) => {
            this.handleFileSelect(e.target.files[0]);
        });

        // Drag and drop
        dropArea.addEventListener('dragover', (e) => {
            e.preventDefault();
            dropArea.classList.add('border-claude-orange');
        });

        dropArea.addEventListener('dragleave', (e) => {
            e.preventDefault();
            dropArea.classList.remove('border-claude-orange');
        });

        dropArea.addEventListener('drop', (e) => {
            e.preventDefault();
            dropArea.classList.remove('border-claude-orange');
            this.handleFileSelect(e.dataTransfer.files[0]);
        });
    }

    handleFileSelect(file) {
        if (!file) return;

        if (!file.name.endsWith('.tar.gz')) {
            this.showError('请选择 .tar.gz 格式的文件');
            return;
        }

        if (file.size > 100 * 1024 * 1024) { // 100MB
            this.showError('文件大小不能超过 100MB');
            return;
        }

        const dropArea = document.getElementById('dropArea');
        const fileInfo = document.getElementById('fileInfo');
        const fileName = document.getElementById('fileName');
        const fileSize = document.getElementById('fileSize');
        const submitBtn = document.getElementById('submitUpload');

        dropArea.classList.add('hidden');
        fileInfo.classList.remove('hidden');
        fileName.textContent = file.name;
        fileSize.textContent = this.formatFileSize(file.size);
        submitBtn.disabled = false;
    }

    async uploadPlugin() {
        const fileInput = document.getElementById('pluginFile');
        const file = fileInput.files[0];
        
        if (!file) {
            this.showError('请选择要上传的文件');
            return;
        }

        const submitBtn = document.getElementById('submitUpload');
        const uploadText = document.getElementById('uploadText');
        const uploadSpinner = document.getElementById('uploadSpinner');

        submitBtn.disabled = true;
        uploadText.textContent = '上传中...';
        uploadSpinner.classList.remove('hidden');

        try {
            // Create FormData for file upload
            const formData = new FormData();
            formData.append('plugin_file', file);
            
            // Prepare headers with authentication
            const headers = {};
            if (this.authToken) {
                headers['Authorization'] = `Bearer ${this.authToken}`;
            }
            
            const response = await fetch(`${this.baseURL}/plugins/upload`, {
                method: 'POST',
                headers,
                body: formData
                // Note: Don't set Content-Type header, let browser set it with boundary
            });
            
            let data;
            const contentType = response.headers.get('content-type');
            if (contentType && contentType.includes('application/json')) {
                data = await response.json();
            } else {
                const text = await response.text();
                console.error('Upload response is not JSON:', text);
                throw new Error(`服务器返回非JSON响应: ${text.substring(0, 100)}...`);
            }
            
            if (response.ok && data.success) {
                this.showSuccess('插件上传成功！');
                this.hideUploadModal();
                this.resetUploadForm();
                this.loadPlugins(); // Refresh plugin list
            } else {
                throw new Error(data.error || `HTTP ${response.status}: Upload failed`);
            }

        } catch (error) {
            console.error('Upload failed:', error);
            if (error.message.includes('401') || error.message.includes('unauthorized')) {
                this.showError('请先登录后再上传插件');
            } else {
                this.showError(`上传失败: ${error.message}`);
            }
        } finally {
            submitBtn.disabled = false;
            uploadText.textContent = '上传插件';
            uploadSpinner.classList.add('hidden');
        }
    }

    resetUploadForm() {
        const fileInput = document.getElementById('pluginFile');
        const dropArea = document.getElementById('dropArea');
        const fileInfo = document.getElementById('fileInfo');
        const submitBtn = document.getElementById('submitUpload');

        fileInput.value = '';
        dropArea.classList.remove('hidden');
        fileInfo.classList.add('hidden');
        submitBtn.disabled = true;
    }

    async downloadPlugin(pluginId) {
        try {
            // Mock download - replace with actual API call
            const downloadUrl = `${this.baseURL}/plugins/${pluginId}/download`;
            
            // Create temporary link and trigger download
            const link = document.createElement('a');
            link.href = downloadUrl;
            link.download = `${pluginId}.tar.gz`;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);

            this.showSuccess('下载已开始');

        } catch (error) {
            console.error('Download failed:', error);
            this.showError('下载失败，请稍后重试');
        }
    }

    showLoading() {
        document.getElementById('loadingState').classList.remove('hidden');
        document.getElementById('pluginGrid').style.opacity = '0.5';
    }

    hideLoading() {
        document.getElementById('loadingState').classList.add('hidden');
        document.getElementById('pluginGrid').style.opacity = '1';
    }

    showPluginModal() {
        document.getElementById('pluginModal').classList.remove('hidden');
        document.body.style.overflow = 'hidden';
    }

    hidePluginModal() {
        document.getElementById('pluginModal').classList.add('hidden');
        document.body.style.overflow = 'auto';
    }

    showUploadModal() {
        document.getElementById('uploadModal').classList.remove('hidden');
        document.body.style.overflow = 'hidden';
    }

    hideUploadModal() {
        document.getElementById('uploadModal').classList.add('hidden');
        document.body.style.overflow = 'auto';
        this.resetUploadForm();
    }

    showError(message) {
        this.showNotification(message, 'error');
    }

    showSuccess(message) {
        this.showNotification(message, 'success');
    }

    showNotification(message, type = 'info') {
        const notification = document.createElement('div');
        notification.className = `fixed top-4 right-4 px-6 py-3 rounded-lg text-white z-50 transform transition-all duration-300 ${
            type === 'error' ? 'bg-red-500' : 
            type === 'success' ? 'bg-green-500' : 
            'bg-blue-500'
        }`;
        notification.textContent = message;

        document.body.appendChild(notification);

        // Animate in
        setTimeout(() => {
            notification.style.transform = 'translateX(0)';
        }, 100);

        // Remove after 3 seconds
        setTimeout(() => {
            notification.style.transform = 'translateX(100%)';
            setTimeout(() => {
                document.body.removeChild(notification);
            }, 300);
        }, 3000);
    }

    timeAgo(dateString) {
        const date = new Date(dateString);
        const now = new Date();
        const diffInSeconds = Math.floor((now - date) / 1000);

        if (diffInSeconds < 60) return '刚刚';
        if (diffInSeconds < 3600) return `${Math.floor(diffInSeconds / 60)}分钟前`;
        if (diffInSeconds < 86400) return `${Math.floor(diffInSeconds / 3600)}小时前`;
        if (diffInSeconds < 604800) return `${Math.floor(diffInSeconds / 86400)}天前`;
        if (diffInSeconds < 2592000) return `${Math.floor(diffInSeconds / 604800)}周前`;
        if (diffInSeconds < 31536000) return `${Math.floor(diffInSeconds / 2592000)}月前`;
        return `${Math.floor(diffInSeconds / 31536000)}年前`;
    }

    // Rating functionality
    showRatingModal(pluginId) {
        this.currentRatingPluginId = pluginId;
        const modal = document.getElementById('ratingModal');
        modal.classList.remove('hidden');
        
        // Reset form
        document.getElementById('selectedRating').value = '';
        document.getElementById('reviewText').value = '';
        this.updateRatingStars(0);
        
        // Add event listeners
        this.setupRatingModalEvents();
    }
    
    hideRatingModal() {
        const modal = document.getElementById('ratingModal');
        modal.classList.add('hidden');
        this.currentRatingPluginId = null;
    }
    
    setupRatingModalEvents() {
        // Close modal events
        document.getElementById('closeRatingModal').onclick = () => this.hideRatingModal();
        document.getElementById('cancelRating').onclick = () => this.hideRatingModal();
        
        // Star click events
        document.querySelectorAll('.rating-star').forEach(star => {
            star.onclick = (e) => {
                e.preventDefault();
                const rating = parseInt(star.dataset.rating);
                document.getElementById('selectedRating').value = rating;
                this.updateRatingStars(rating);
            };
        });
        
        // Form submission
        document.getElementById('ratingForm').onsubmit = (e) => {
            e.preventDefault();
            this.submitRating();
        };
    }
    
    updateRatingStars(rating) {
        document.querySelectorAll('.rating-star').forEach((star, index) => {
            if (index < rating) {
                star.classList.remove('text-gray-300');
                star.classList.add('text-yellow-400');
            } else {
                star.classList.remove('text-yellow-400');
                star.classList.add('text-gray-300');
            }
        });
    }
    
    async submitRating() {
        const rating = parseInt(document.getElementById('selectedRating').value);
        const review = document.getElementById('reviewText').value.trim();
        
        if (!rating || rating < 1 || rating > 5) {
            this.showError('请选择1-5星评分');
            return;
        }
        
        const submitBtn = document.getElementById('submitRating');
        const submitText = document.getElementById('ratingSubmitText');
        const spinner = document.getElementById('ratingSpinner');
        
        try {
            submitBtn.disabled = true;
            submitText.textContent = '提交中...';
            spinner.classList.remove('hidden');
            
            const response = await fetch(`${this.baseURL}/plugins/${this.currentRatingPluginId}/ratings`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${this.authToken}`
                },
                body: JSON.stringify({
                    rating: rating,
                    review: review || null
                })
            });
            
            const data = await response.json();
            
            if (response.ok && data.success) {
                this.showSuccess('评分提交成功！');
                this.hideRatingModal();
                // Refresh plugin details if modal is open
                if (this.currentRatingPluginId) {
                    this.showPluginDetail(this.currentRatingPluginId);
                }
            } else {
                throw new Error(data.error || '评分提交失败');
            }
            
        } catch (error) {
            console.error('Rating submission failed:', error);
            this.showError('评分提交失败：' + error.message);
        } finally {
            submitBtn.disabled = false;
            submitText.textContent = '提交评分';
            spinner.classList.add('hidden');
        }
    }

    formatFileSize(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }
}

// Initialize the marketplace when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.marketplace = new PluginMarketplace();
});