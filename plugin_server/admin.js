// GeekTools Admin Panel Frontend
class AdminPanel {
    constructor() {
        this.baseURL = '/api/v1';
        this.authToken = localStorage.getItem('auth_token');
        this.currentUser = JSON.parse(localStorage.getItem('current_user') || 'null');
        this.currentTab = 'dashboard';
        
        // Pagination state
        this.usersPagination = { page: 1, limit: 20 };
        this.pluginsPagination = { page: 1, limit: 20 };
        this.activitiesPagination = { page: 1, limit: 20 };
        
        this.init();
    }

    init() {
        this.checkAuth();
        this.bindEvents();
        this.loadDashboard();
    }

    checkAuth() {
        if (!this.authToken || !this.currentUser) {
            alert('请先登录');
            window.location.href = 'index.html';
            return;
        }

        // Check if user is admin - this should be checked on server side too
        document.getElementById('adminEmail').textContent = this.currentUser.email;
    }

    bindEvents() {
        // Tab switching
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const tabId = e.target.id.replace('Tab', '');
                this.switchTab(tabId);
            });
        });

        // Logout
        document.getElementById('logoutBtn').addEventListener('click', () => {
            this.logout();
        });

        // Refresh buttons
        document.getElementById('refreshUsersBtn').addEventListener('click', () => {
            this.loadUsers();
        });

        document.getElementById('refreshPluginsBtn').addEventListener('click', () => {
            this.loadPlugins();
        });

        document.getElementById('refreshActivitiesBtn').addEventListener('click', () => {
            this.loadActivities();
        });

        // SQL console
        document.getElementById('executeSqlBtn').addEventListener('click', () => {
            this.executeSql();
        });

        document.getElementById('clearSqlBtn').addEventListener('click', () => {
            document.getElementById('sqlQuery').value = '';
            document.getElementById('sqlResults').classList.add('hidden');
        });

        // Edit email modal
        document.getElementById('closeEditEmailModal').addEventListener('click', () => {
            this.hideEditEmailModal();
        });

        document.getElementById('cancelEditEmail').addEventListener('click', () => {
            this.hideEditEmailModal();
        });

        document.getElementById('editEmailForm').addEventListener('submit', (e) => {
            e.preventDefault();
            this.saveUserEmail();
        });

        // User filter
        document.getElementById('userFilter').addEventListener('change', (e) => {
            this.activitiesPagination.page = 1;
            this.loadActivities();
        });
    }

    switchTab(tabName) {
        // Remove active class from all tabs
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.classList.remove('tab-active');
        });

        // Hide all tab content
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.add('hidden');
        });

        // Activate selected tab
        document.getElementById(tabName + 'Tab').classList.add('tab-active');
        document.getElementById(tabName + 'Content').classList.remove('hidden');

        this.currentTab = tabName;

        // Load content based on tab
        switch (tabName) {
            case 'dashboard':
                this.loadDashboard();
                break;
            case 'users':
                this.loadUsers();
                break;
            case 'plugins':
                this.loadPlugins();
                break;
            case 'activities':
                this.loadActivities();
                this.loadUserFilter();
                break;
            case 'sql':
                // SQL console is already ready
                break;
        }
    }

    async makeAuthenticatedRequest(url, options = {}) {
        const headers = {
            'Authorization': `Bearer ${this.authToken}`,
            'Content-Type': 'application/json',
            ...options.headers
        };

        const response = await fetch(url, {
            ...options,
            headers
        });

        if (response.status === 401 || response.status === 403) {
            alert('认证失败或权限不足');
            this.logout();
            return null;
        }

        return response;
    }

    async loadDashboard() {
        try {
            const response = await this.makeAuthenticatedRequest(`${this.baseURL}/admin/dashboard`);
            if (!response) return;

            const data = await response.json();
            if (data.success) {
                const stats = data.data;
                document.getElementById('totalUsers').textContent = stats.total_users.toLocaleString();
                document.getElementById('totalPlugins').textContent = stats.total_plugins.toLocaleString();
                document.getElementById('totalDownloads').textContent = stats.total_downloads.toLocaleString();
                document.getElementById('activeSessions').textContent = stats.active_sessions.toLocaleString();

                this.renderRecentLogins(stats.recent_logins);
                this.renderRecentSqlExecutions(stats.recent_sql_executions);
            }
        } catch (error) {
            console.error('Failed to load dashboard:', error);
            this.showError('加载仪表板数据失败');
        }
    }

    renderRecentLogins(logins) {
        const container = document.getElementById('recentLogins');
        if (logins.length === 0) {
            container.innerHTML = '<p class="text-gray-500 text-sm">暂无登录记录</p>';
            return;
        }

        container.innerHTML = logins.slice(0, 5).map(login => `
            <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                <div>
                    <p class="font-medium text-sm">${login.email}</p>
                    <p class="text-xs text-gray-500">${login.ip_address || 'N/A'} • ${this.timeAgo(login.login_time)}</p>
                </div>
                <div class="flex items-center">
                    ${login.is_successful 
                        ? '<i class="fas fa-check-circle text-green-500"></i>' 
                        : '<i class="fas fa-times-circle text-red-500"></i>'
                    }
                </div>
            </div>
        `).join('');
    }

    renderRecentSqlExecutions(executions) {
        const container = document.getElementById('recentSqlExecutions');
        if (executions.length === 0) {
            container.innerHTML = '<p class="text-gray-500 text-sm">暂无SQL执行记录</p>';
            return;
        }

        container.innerHTML = executions.slice(0, 5).map(exec => `
            <div class="p-3 bg-gray-50 rounded-lg">
                <div class="flex items-center justify-between mb-1">
                    <p class="font-medium text-sm">${exec.admin_email}</p>
                    <div class="flex items-center space-x-2">
                        <span class="text-xs text-gray-500">${exec.execution_time_ms}ms</span>
                        ${exec.is_successful 
                            ? '<i class="fas fa-check-circle text-green-500"></i>' 
                            : '<i class="fas fa-times-circle text-red-500"></i>'
                        }
                    </div>
                </div>
                <p class="text-xs text-gray-600 font-mono truncate">${exec.sql_query}</p>
                <p class="text-xs text-gray-500 mt-1">${this.timeAgo(exec.executed_at)}</p>
            </div>
        `).join('');
    }

    async loadUsers() {
        try {
            const params = new URLSearchParams({
                page: this.usersPagination.page.toString(),
                limit: this.usersPagination.limit.toString()
            });

            const response = await this.makeAuthenticatedRequest(`${this.baseURL}/admin/users?${params}`);
            if (!response) return;

            const data = await response.json();
            if (data.success) {
                this.renderUsers(data.data.users);
                this.renderUsersPagination(data.data.total_count);
            }
        } catch (error) {
            console.error('Failed to load users:', error);
            this.showError('加载用户数据失败');
        }
    }

    renderUsers(users) {
        const tbody = document.getElementById('usersTableBody');
        tbody.innerHTML = users.map(user => `
            <tr class="border-b border-gray-100 hover:bg-gray-50">
                <td class="py-3 px-4">${user.id}</td>
                <td class="py-3 px-4">${user.username}</td>
                <td class="py-3 px-4">${user.email}</td>
                <td class="py-3 px-4">
                    <span class="px-2 py-1 text-xs rounded-full ${
                        user.role === 'admin' ? 'bg-red-100 text-red-700' : 'bg-gray-100 text-gray-700'
                    }">
                        ${user.role === 'admin' ? '管理员' : '用户'}
                    </span>
                </td>
                <td class="py-3 px-4">
                    <span class="px-2 py-1 text-xs rounded-full ${
                        user.is_active ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-700'
                    }">
                        ${user.is_active ? '活跃' : '禁用'}
                    </span>
                </td>
                <td class="py-3 px-4 text-sm text-gray-600">
                    ${user.last_login ? this.timeAgo(user.last_login) : '从未登录'}
                </td>
                <td class="py-3 px-4">
                    <div class="flex space-x-2">
                        <button onclick="adminPanel.showEditEmailModal(${user.id}, '${user.username}', '${user.email}')" 
                                class="text-blue-600 hover:text-blue-800 text-sm">
                            <i class="fas fa-edit mr-1"></i>编辑邮箱
                        </button>
                        ${user.is_active ? 
                            `<button onclick="adminPanel.banUser(${user.id}, '${user.username}')" 
                                     class="text-red-600 hover:text-red-800 text-sm">
                                <i class="fas fa-ban mr-1"></i>封禁
                             </button>` :
                            `<button onclick="adminPanel.unbanUser(${user.id}, '${user.username}')" 
                                     class="text-green-600 hover:text-green-800 text-sm">
                                <i class="fas fa-check mr-1"></i>解封
                             </button>`
                        }
                    </div>
                </td>
            </tr>
        `).join('');
    }

    renderUsersPagination(totalCount) {
        const totalPages = Math.ceil(totalCount / this.usersPagination.limit);
        const container = document.getElementById('usersPagination');
        
        if (totalPages <= 1) {
            container.innerHTML = '';
            return;
        }

        let pagination = `
            <div class="flex items-center space-x-2">
                <button onclick="adminPanel.goToUsersPage(${this.usersPagination.page - 1})" 
                        ${this.usersPagination.page === 1 ? 'disabled' : ''}
                        class="px-3 py-2 border border-gray-200 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed">
                    <i class="fas fa-chevron-left"></i>
                </button>
        `;

        // Show page numbers
        const start = Math.max(1, this.usersPagination.page - 2);
        const end = Math.min(totalPages, this.usersPagination.page + 2);

        for (let i = start; i <= end; i++) {
            pagination += `
                <button onclick="adminPanel.goToUsersPage(${i})" 
                        class="px-3 py-2 border border-gray-200 rounded-lg hover:bg-gray-50 ${
                            i === this.usersPagination.page ? 'bg-claude-orange text-white' : ''
                        }">
                    ${i}
                </button>
            `;
        }

        pagination += `
                <button onclick="adminPanel.goToUsersPage(${this.usersPagination.page + 1})" 
                        ${this.usersPagination.page === totalPages ? 'disabled' : ''}
                        class="px-3 py-2 border border-gray-200 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed">
                    <i class="fas fa-chevron-right"></i>
                </button>
            </div>
        `;

        container.innerHTML = pagination;
    }

    goToUsersPage(page) {
        this.usersPagination.page = page;
        this.loadUsers();
    }

    async loadUserFilter() {
        try {
            const response = await this.makeAuthenticatedRequest(`${this.baseURL}/admin/users?limit=1000`);
            if (!response) return;

            const data = await response.json();
            if (data.success) {
                const select = document.getElementById('userFilter');
                select.innerHTML = '<option value="">所有用户</option>' + 
                    data.data.users.map(user => `
                        <option value="${user.id}">${user.username} (${user.email})</option>
                    `).join('');
            }
        } catch (error) {
            console.error('Failed to load user filter:', error);
        }
    }

    async loadActivities() {
        try {
            const params = new URLSearchParams({
                page: this.activitiesPagination.page.toString(),
                limit: this.activitiesPagination.limit.toString()
            });

            const userId = document.getElementById('userFilter').value;
            if (userId) {
                params.append('user_id', userId);
            }

            const response = await this.makeAuthenticatedRequest(`${this.baseURL}/admin/login-activities?${params}`);
            if (!response) return;

            const data = await response.json();
            if (data.success) {
                this.renderActivities(data.data.activities);
                this.renderActivitiesPagination(data.data.total_count);
            }
        } catch (error) {
            console.error('Failed to load activities:', error);
            this.showError('加载活动数据失败');
        }
    }

    renderActivities(activities) {
        const tbody = document.getElementById('activitiesTableBody');
        tbody.innerHTML = activities.map(activity => `
            <tr class="border-b border-gray-100 hover:bg-gray-50">
                <td class="py-3 px-4 text-sm">${this.formatDateTime(activity.login_time)}</td>
                <td class="py-3 px-4">
                    <div>
                        <p class="font-medium text-sm">${activity.email}</p>
                        <p class="text-xs text-gray-500">ID: ${activity.user_id}</p>
                    </div>
                </td>
                <td class="py-3 px-4 text-sm">${activity.ip_address || 'N/A'}</td>
                <td class="py-3 px-4 text-sm max-w-xs truncate" title="${activity.user_agent || 'N/A'}">
                    ${activity.user_agent || 'N/A'}
                </td>
                <td class="py-3 px-4">
                    ${activity.is_successful 
                        ? '<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">成功</span>'
                        : `<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full" title="${activity.failure_reason || ''}">失败</span>`
                    }
                </td>
                <td class="py-3 px-4 text-sm">${activity.login_method}</td>
            </tr>
        `).join('');
    }

    renderActivitiesPagination(totalCount) {
        const totalPages = Math.ceil(totalCount / this.activitiesPagination.limit);
        const container = document.getElementById('activitiesPagination');
        
        if (totalPages <= 1) {
            container.innerHTML = '';
            return;
        }

        let pagination = `
            <div class="flex items-center space-x-2">
                <button onclick="adminPanel.goToActivitiesPage(${this.activitiesPagination.page - 1})" 
                        ${this.activitiesPagination.page === 1 ? 'disabled' : ''}
                        class="px-3 py-2 border border-gray-200 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed">
                    <i class="fas fa-chevron-left"></i>
                </button>
        `;

        const start = Math.max(1, this.activitiesPagination.page - 2);
        const end = Math.min(totalPages, this.activitiesPagination.page + 2);

        for (let i = start; i <= end; i++) {
            pagination += `
                <button onclick="adminPanel.goToActivitiesPage(${i})" 
                        class="px-3 py-2 border border-gray-200 rounded-lg hover:bg-gray-50 ${
                            i === this.activitiesPagination.page ? 'bg-claude-orange text-white' : ''
                        }">
                    ${i}
                </button>
            `;
        }

        pagination += `
                <button onclick="adminPanel.goToActivitiesPage(${this.activitiesPagination.page + 1})" 
                        ${this.activitiesPagination.page === totalPages ? 'disabled' : ''}
                        class="px-3 py-2 border border-gray-200 rounded-lg hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed">
                    <i class="fas fa-chevron-right"></i>
                </button>
            </div>
        `;

        container.innerHTML = pagination;
    }

    goToActivitiesPage(page) {
        this.activitiesPagination.page = page;
        this.loadActivities();
    }

    async executeSql() {
        const query = document.getElementById('sqlQuery').value.trim();
        if (!query) {
            this.showError('请输入SQL查询');
            return;
        }

        const executeBtn = document.getElementById('executeSqlBtn');
        const spinner = document.getElementById('sqlSpinner');
        
        executeBtn.disabled = true;
        spinner.classList.remove('hidden');

        try {
            const response = await this.makeAuthenticatedRequest(`${this.baseURL}/admin/sql/execute`, {
                method: 'POST',
                body: JSON.stringify({ sql_query: query })
            });

            if (!response) return;

            const data = await response.json();
            
            if (data.success) {
                this.displaySqlResults(data.data);
            } else {
                this.showError(`SQL执行失败: ${data.error}`);
            }
        } catch (error) {
            console.error('SQL execution failed:', error);
            this.showError(`SQL执行失败: ${error.message}`);
        } finally {
            executeBtn.disabled = false;
            spinner.classList.add('hidden');
        }
    }

    displaySqlResults(result) {
        const resultsDiv = document.getElementById('sqlResults');
        const contentDiv = document.getElementById('sqlResultsContent');
        const infoDiv = document.getElementById('sqlExecutionInfo');

        resultsDiv.classList.remove('hidden');

        if (result.error_message) {
            contentDiv.innerHTML = `
                <div class="p-4 bg-red-50 text-red-700">
                    <i class="fas fa-exclamation-triangle mr-2"></i>
                    错误: ${result.error_message}
                </div>
            `;
        } else if (result.data && result.data.length > 0) {
            // Display table results
            const keys = Object.keys(result.data[0]);
            const tableHTML = `
                <table class="w-full text-sm">
                    <thead class="bg-gray-50">
                        <tr>
                            ${keys.map(key => `<th class="text-left py-2 px-3 font-medium text-gray-600">${key}</th>`).join('')}
                        </tr>
                    </thead>
                    <tbody>
                        ${result.data.map(row => `
                            <tr class="border-b border-gray-100">
                                ${keys.map(key => `<td class="py-2 px-3">${this.formatSqlValue(row[key])}</td>`).join('')}
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            `;
            contentDiv.innerHTML = tableHTML;
        } else {
            contentDiv.innerHTML = `
                <div class="p-4 bg-green-50 text-green-700">
                    <i class="fas fa-check mr-2"></i>
                    查询执行成功，没有返回数据
                </div>
            `;
        }

        infoDiv.innerHTML = `
            执行时间: ${result.execution_time_ms}ms | 
            影响行数: ${result.rows_affected || 0} | 
            状态: ${result.is_successful ? '成功' : '失败'}
        `;
    }

    formatSqlValue(value) {
        if (value === null || value === undefined) {
            return '<span class="text-gray-400 italic">NULL</span>';
        }
        if (typeof value === 'string' && value.length > 100) {
            return `<span title="${value}">${value.substring(0, 100)}...</span>`;
        }
        return value;
    }

    showEditEmailModal(userId, username, currentEmail) {
        document.getElementById('editUserId').value = userId;
        document.getElementById('editUserName').value = username;
        document.getElementById('editCurrentEmail').value = currentEmail;
        document.getElementById('editNewEmail').value = '';
        document.getElementById('editReason').value = '';
        
        document.getElementById('editEmailModal').classList.remove('hidden');
        document.body.style.overflow = 'hidden';
    }

    hideEditEmailModal() {
        document.getElementById('editEmailModal').classList.add('hidden');
        document.body.style.overflow = 'auto';
    }

    async saveUserEmail() {
        const userId = parseInt(document.getElementById('editUserId').value);
        const newEmail = document.getElementById('editNewEmail').value.trim();
        const reason = document.getElementById('editReason').value.trim();

        if (!newEmail) {
            this.showError('请输入新邮箱地址');
            return;
        }

        const saveBtn = document.getElementById('saveEmailBtn');
        const saveText = document.getElementById('saveEmailText');
        const saveSpinner = document.getElementById('saveEmailSpinner');

        saveBtn.disabled = true;
        saveText.textContent = '保存中...';
        saveSpinner.classList.remove('hidden');

        try {
            const response = await this.makeAuthenticatedRequest(`${this.baseURL}/admin/users/update-email`, {
                method: 'POST',
                body: JSON.stringify({
                    user_id: userId,
                    new_email: newEmail,
                    reason: reason || null
                })
            });

            if (!response) return;

            const data = await response.json();
            
            if (data.success) {
                this.showSuccess('用户邮箱更新成功');
                this.hideEditEmailModal();
                this.loadUsers(); // Refresh user list
            } else {
                this.showError(`更新失败: ${data.error}`);
            }
        } catch (error) {
            console.error('Failed to update user email:', error);
            this.showError(`更新失败: ${error.message}`);
        } finally {
            saveBtn.disabled = false;
            saveText.textContent = '保存修改';
            saveSpinner.classList.add('hidden');
        }
    }

    logout() {
        localStorage.removeItem('auth_token');
        localStorage.removeItem('current_user');
        window.location.href = 'index.html';
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

    formatDateTime(dateString) {
        const date = new Date(dateString);
        return date.toLocaleString('zh-CN', {
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit'
        });
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

        setTimeout(() => {
            notification.style.transform = 'translateX(0)';
        }, 100);

        setTimeout(() => {
            notification.style.transform = 'translateX(100%)';
            setTimeout(() => {
                document.body.removeChild(notification);
            }, 300);
        }, 3000);
    }

    // Plugin management methods
    async loadPlugins() {
        try {
            document.getElementById('pluginsLoading').classList.remove('hidden');
            document.getElementById('pluginsEmpty').classList.add('hidden');
            
            const response = await fetch(`${this.baseURL}/admin/plugins?page=${this.pluginsPagination.page}&limit=${this.pluginsPagination.limit}`, {
                headers: {
                    'Authorization': `Bearer ${this.authToken}`,
                    'Content-Type': 'application/json',
                }
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const result = await response.json();
            
            if (result.success) {
                this.renderPlugins(result.data.plugins);
                this.updatePluginsPagination(result.data.total_count);
            } else {
                throw new Error(result.message || 'Failed to load plugins');
            }
        } catch (error) {
            console.error('Error loading plugins:', error);
            this.showNotification('加载插件列表失败: ' + error.message, 'error');
            document.getElementById('pluginsEmpty').classList.remove('hidden');
        } finally {
            document.getElementById('pluginsLoading').classList.add('hidden');
        }
    }

    renderPlugins(plugins) {
        const tbody = document.getElementById('pluginsTableBody');
        
        if (!plugins || plugins.length === 0) {
            document.getElementById('pluginsEmpty').classList.remove('hidden');
            tbody.innerHTML = '';
            return;
        }

        tbody.innerHTML = plugins.map(plugin => `
            <tr class="hover:bg-gray-50">
                <td class="px-4 py-4">
                    <div class="flex items-center">
                        <div class="w-10 h-10 bg-gradient-to-br from-claude-orange to-orange-600 rounded-lg flex items-center justify-center mr-3">
                            <i class="fas fa-puzzle-piece text-white"></i>
                        </div>
                        <div>
                            <div class="font-semibold text-gray-900">${this.escapeHtml(plugin.name || plugin.id)}</div>
                            <div class="text-sm text-gray-500">${this.escapeHtml(plugin.description || 'No description')}</div>
                        </div>
                    </div>
                </td>
                <td class="px-4 py-4 text-sm text-gray-900">${this.escapeHtml(plugin.author || 'Unknown')}</td>
                <td class="px-4 py-4 text-sm text-gray-900">${this.escapeHtml(plugin.current_version || 'N/A')}</td>
                <td class="px-4 py-4 text-sm text-gray-900">${plugin.downloads || 0}</td>
                <td class="px-4 py-4">
                    <span class="px-2 py-1 text-xs font-medium rounded-full ${
                        plugin.is_active ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
                    }">
                        ${plugin.is_active ? '活跃' : '已禁用'}
                    </span>
                </td>
                <td class="px-4 py-4">
                    <div class="flex items-center space-x-2">
                        <button onclick="adminPanel.deletePlugin('${plugin.id}', '${this.escapeHtml(plugin.name || plugin.id)}')" 
                                class="px-3 py-1 bg-red-500 text-white text-xs rounded hover:bg-red-600 transition-colors">
                            <i class="fas fa-trash mr-1"></i>删除
                        </button>
                    </div>
                </td>
            </tr>
        `).join('');
    }

    updatePluginsPagination(totalCount) {
        const totalPages = Math.ceil(totalCount / this.pluginsPagination.limit);
        const startItem = (this.pluginsPagination.page - 1) * this.pluginsPagination.limit + 1;
        const endItem = Math.min(this.pluginsPagination.page * this.pluginsPagination.limit, totalCount);

        document.getElementById('pluginsPageStart').textContent = startItem;
        document.getElementById('pluginsPageEnd').textContent = endItem;
        document.getElementById('pluginsTotalCount').textContent = totalCount;
        document.getElementById('pluginsCurrentPage').textContent = this.pluginsPagination.page;

        const prevBtn = document.getElementById('pluginsPrevPage');
        const nextBtn = document.getElementById('pluginsNextPage');

        prevBtn.disabled = this.pluginsPagination.page <= 1;
        nextBtn.disabled = this.pluginsPagination.page >= totalPages;

        // Show pagination if there are items
        if (totalCount > 0) {
            document.getElementById('pluginsPagination').classList.remove('hidden');
        } else {
            document.getElementById('pluginsPagination').classList.add('hidden');
        }
    }

    async deletePlugin(pluginId, pluginName) {
        const reason = prompt(`确定要删除插件 "${pluginName}" 吗？\n\n请输入删除原因（可选）:`);
        
        if (reason === null) {
            return; // User cancelled
        }

        try {
            const response = await fetch(`${this.baseURL}/admin/plugins/delete`, {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${this.authToken}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    plugin_id: pluginId,
                    reason: reason.trim() || null
                })
            });

            const result = await response.json();
            
            if (response.ok && result.success) {
                this.showNotification('插件删除成功', 'success');
                this.loadPlugins(); // Refresh the list
            } else {
                throw new Error(result.message || 'Failed to delete plugin');
            }
        } catch (error) {
            console.error('Error deleting plugin:', error);
            this.showNotification('删除插件失败: ' + error.message, 'error');
        }
    }

    async banUser(userId, username) {
        const reason = prompt(`确定要封禁用户 "${username}" 吗？\n\n请输入封禁原因（可选）:`);
        
        if (reason === null) {
            return; // User cancelled
        }

        try {
            const response = await fetch(`${this.baseURL}/admin/users/ban`, {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${this.authToken}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    user_id: userId,
                    reason: reason.trim() || null,
                    ban_duration_days: null // Permanent ban, could be made configurable
                })
            });

            const result = await response.json();
            
            if (response.ok && result.success) {
                this.showNotification('用户封禁成功', 'success');
                this.loadUsers(); // Refresh the list
            } else {
                throw new Error(result.message || 'Failed to ban user');
            }
        } catch (error) {
            console.error('Error banning user:', error);
            this.showNotification('封禁用户失败: ' + error.message, 'error');
        }
    }

    async unbanUser(userId, username) {
        const reason = prompt(`确定要解封用户 "${username}" 吗？\n\n请输入解封原因（可选）:`);
        
        if (reason === null) {
            return; // User cancelled
        }

        try {
            const response = await fetch(`${this.baseURL}/admin/users/unban`, {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${this.authToken}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    user_id: userId,
                    reason: reason.trim() || null
                })
            });

            const result = await response.json();
            
            if (response.ok && result.success) {
                this.showNotification('用户解封成功', 'success');
                this.loadUsers(); // Refresh the list
            } else {
                throw new Error(result.message || 'Failed to unban user');
            }
        } catch (error) {
            console.error('Error unbanning user:', error);
            this.showNotification('解封用户失败: ' + error.message, 'error');
        }
    }

    escapeHtml(text) {
        const map = {
            '&': '&amp;',
            '<': '&lt;',
            '>': '&gt;',
            '"': '&quot;',
            "'": '&#039;'
        };
        return text.toString().replace(/[&<>"']/g, function(m) { return map[m]; });
    }
}

// Initialize admin panel when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.adminPanel = new AdminPanel();
});