import { invoke } from '@tauri-apps/api/core';

console.log('Script loaded, invoke:', invoke);

// 应用状态
const state = {
    currentProject: null,
    currentTab: 'storyboard',
    storyboards: [],
    characters: [],
    scenes: [],
    props: [],
    apis: [],  // API 配置列表
    editingApiIndex: null,  // 当前编辑的 API 索引，null 表示新增
    chatHistory: []  // 聊天历史记录
};

// DOM 元素
const welcomeScreen = document.getElementById('welcome-screen');
const mainScreen = document.getElementById('main-screen');
const apiConfigModal = document.getElementById('api-config-modal');
const apiEditModal = document.getElementById('api-edit-modal');
const storyboardDetailModal = document.getElementById('storyboard-detail-modal');
const projectTitle = document.getElementById('project-title');
const aiSidebar = document.getElementById('ai-sidebar');
const chatMessages = document.getElementById('chat-messages');
const chatInput = document.getElementById('chat-input');
const storyboardTbody = document.getElementById('storyboard-tbody');
const shotCount = document.getElementById('shot-count');

// 初始化
async function init() {
    console.log('init() called!');
    setupEventListeners();
    console.log('Event listeners set up!');

    // 加载配置
    await loadConfig();

    // 加载最近项目列表
    await loadRecentProjects();
}

// 加载配置
async function loadConfig() {
    try {
        const config = await invoke('get_global_config');
        console.log('Config loaded:', config);
        if (config && config.apis) {
            state.apis = config.apis;
        }
    } catch (error) {
        console.error('Failed to load config:', error);
    }
}

// 设置事件监听器
function setupEventListeners() {
    // 欢迎界面按钮
    document.getElementById('new-project-btn').addEventListener('click', handleNewProject);
    document.getElementById('open-project-btn').addEventListener('click', handleOpenProject);
    document.getElementById('settings-btn').addEventListener('click', () => {
        renderApiList();
        showApiConfigModal();
    });

    // 主界面配置按钮
    document.getElementById('main-config-btn').addEventListener('click', () => {
        renderApiList();
        showApiConfigModal();
    });

    // 文件菜单
    const fileMenuBtn = document.getElementById('file-menu-btn');
    const fileMenuDropdown = document.getElementById('file-menu-dropdown');

    fileMenuBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        fileMenuDropdown.classList.toggle('hidden');
    });

    // 点击其他地方关闭菜单
    document.addEventListener('click', () => {
        fileMenuDropdown.classList.add('hidden');
    });

    fileMenuDropdown.addEventListener('click', (e) => {
        e.stopPropagation();
    });

    document.getElementById('menu-new-project').addEventListener('click', () => {
        fileMenuDropdown.classList.add('hidden');
        handleNewProject();
    });

    document.getElementById('menu-open-project').addEventListener('click', () => {
        fileMenuDropdown.classList.add('hidden');
        handleOpenProject();
    });

    // 新建项目弹窗
    document.getElementById('new-project-btn').addEventListener('click', handleNewProject);
    document.getElementById('close-new-project-modal').addEventListener('click', () => {
        document.getElementById('new-project-modal').classList.add('hidden');
    });
    document.getElementById('cancel-new-project').addEventListener('click', () => {
        document.getElementById('new-project-modal').classList.add('hidden');
    });
    document.getElementById('confirm-new-project').addEventListener('click', confirmNewProject);
    document.getElementById('new-project-name').addEventListener('keydown', (e) => {
        if (e.key === 'Enter') confirmNewProject();
        if (e.key === 'Escape') {
            document.getElementById('new-project-modal').classList.add('hidden');
        }
    });

    // 打开项目弹窗
    document.getElementById('open-project-btn').addEventListener('click', handleOpenProject);
    document.getElementById('close-open-project-modal').addEventListener('click', () => {
        document.getElementById('open-project-modal').classList.add('hidden');
    });

    // 项目标题双击重命名
    projectTitle.addEventListener('dblclick', editProjectName);

    // API 配置弹窗
    document.getElementById('close-api-modal').addEventListener('click', hideApiConfigModal);
    document.getElementById('save-api-config').addEventListener('click', handleSaveApiConfig);
    document.getElementById('add-api-btn').addEventListener('click', () => showApiEditModal(null));

    // API 编辑弹窗
    document.getElementById('close-api-edit-modal').addEventListener('click', hideApiEditModal);
    document.getElementById('cancel-api-edit').addEventListener('click', hideApiEditModal);
    document.getElementById('confirm-api-edit').addEventListener('click', handleConfirmApiEdit);

    // 分镜详情弹窗
    document.getElementById('close-detail-modal').addEventListener('click', hideStoryboardDetail);

    // 布局切换
    document.getElementById('two-col-layout').addEventListener('click', () => toggleLayout(false));
    document.getElementById('three-col-layout').addEventListener('click', () => toggleLayout(true));

    // 标签切换
    document.querySelectorAll('[data-tab]').forEach(el => {
        el.addEventListener('click', () => switchTab(el.dataset.tab));
    });

    // 聊天
    document.getElementById('send-chat-btn').addEventListener('click', sendChatMessage);
    chatInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendChatMessage();
        }
    });

    // 剧本上传
    const fileInput = document.getElementById('script-file-input');
    document.getElementById('upload-script-btn').addEventListener('click', () => {
        fileInput.click();
    });
    fileInput.addEventListener('change', handleFileUpload);
}

// 新建项目
async function handleNewProject() {
    const modal = document.getElementById('new-project-modal');
    const nameInput = document.getElementById('new-project-name');
    const errorText = document.getElementById('project-name-error');

    nameInput.value = '';
    errorText.classList.add('hidden');
    modal.classList.remove('hidden');
    nameInput.focus();
}

// 确认新建项目
async function confirmNewProject() {
    const nameInput = document.getElementById('new-project-name');
    const errorText = document.getElementById('project-name-error');
    const modal = document.getElementById('new-project-modal');

    const name = nameInput.value.trim();

    if (!name) {
        errorText.textContent = '请输入项目名称';
        errorText.classList.remove('hidden');
        return;
    }

    // 检查名称是否已存在
    try {
        const exists = await invoke('check_project_name_exists', {
            folderPath: 'D:\\分镜项目',
            projectName: name,
            excludePath: null
        });

        if (exists) {
            errorText.textContent = `项目名称 "${name}" 已存在，请使用其他名称`;
            errorText.classList.remove('hidden');
            return;
        }
    } catch (e) {
        console.error('检查项目名称失败:', e);
    }

    try {
        const projectPath = await invoke('create_project', {
            folderPath: 'D:\\分镜项目',
            projectName: name
        });
        state.currentProject = projectPath;
        projectTitle.textContent = name;  // 设置项目标题
        await loadProjectData();  // 加载项目数据
        modal.classList.add('hidden');
        showMainScreen();
    } catch (error) {
        errorText.textContent = error;
        errorText.classList.remove('hidden');
    }
}

// 打开项目
async function handleOpenProject() {
    const modal = document.getElementById('open-project-modal');
    const listEl = document.getElementById('projects-list');
    const noProjectsMsg = document.getElementById('no-projects-message');

    listEl.innerHTML = '';
    noProjectsMsg.classList.add('hidden');

    try {
        const projects = await invoke('list_projects', {
            folderPath: 'D:\\分镜项目'
        });

        if (projects.length === 0) {
            noProjectsMsg.classList.remove('hidden');
        } else {
            projects.forEach(project => {
                const item = document.createElement('div');
                item.className = 'p-3 border border-slate-200 rounded-lg hover:bg-slate-50 cursor-pointer transition-colors';
                item.innerHTML = `
                    <div class="flex items-center justify-between">
                        <div>
                            <h4 class="font-semibold text-sm text-slate-800">${project.name}</h4>
                            <p class="text-xs text-slate-500 mt-1">
                                ${project.storyboard_count} 镜头 · ${new Date(project.modified_at).toLocaleDateString()}
                            </p>
                        </div>
                        <span class="material-symbols-outlined text-slate-400">folder_open</span>
                    </div>
                `;
                item.addEventListener('click', () => openProject(project));
                listEl.appendChild(item);
            });
        }

        modal.classList.remove('hidden');
    } catch (error) {
        alert('加载项目列表失败: ' + error);
    }
}

// 打开指定项目
async function openProject(project) {
    try {
        state.currentProject = project.path;
        projectTitle.textContent = project.name;
        await loadProjectData();

        // 关闭弹窗
        document.getElementById('open-project-modal').classList.add('hidden');
        showMainScreen();
    } catch (error) {
        alert('打开项目失败: ' + error);
    }
}

// 加载最近项目列表
async function loadRecentProjects() {
    const container = document.getElementById('recent-projects');
    const listEl = document.getElementById('recent-projects-list');

    try {
        const projects = await invoke('list_projects', {
            folderPath: 'D:\\分镜项目'
        });

        if (projects.length > 0) {
            container.classList.remove('hidden');
            listEl.innerHTML = '';

            projects.slice(0, 5).forEach(project => {
                const item = document.createElement('div');
                item.className = 'p-2 bg-white border border-slate-200 rounded-lg hover:bg-slate-50 cursor-pointer transition-colors text-left';
                item.innerHTML = `
                    <h4 class="font-medium text-sm text-slate-800">${project.name}</h4>
                    <p class="text-xs text-slate-500">${project.storyboard_count} 镜头</p>
                `;
                item.addEventListener('click', () => openProject(project));
                listEl.appendChild(item);
            });
        }
    } catch (error) {
        console.error('加载最近项目失败:', error);
    }
}

// 编辑项目名称
async function editProjectName() {
    if (!state.currentProject) return;

    const currentName = projectTitle.textContent;
    let newName = prompt('请输入新的项目名称：', currentName);

    if (!newName || newName.trim() === '' || newName === currentName) {
        return;
    }

    newName = newName.trim();

    try {
        // 调用后端命令更新项目名称（后端会检查重名）
        await invoke('update_project_name', {
            folderPath: state.currentProject,
            name: newName
        });

        // 更新界面显示
        projectTitle.textContent = newName;
    } catch (error) {
        alert('更新项目名称失败: ' + error);
    }
}

// 加载项目数据
async function loadProjectData() {
    try {
        state.storyboards = await invoke('get_storyboards', { folderPath: state.currentProject });
        state.characters = await invoke('get_characters', { folderPath: state.currentProject });
        state.scenes = await invoke('get_scenes', { folderPath: state.currentProject });
        state.props = await invoke('get_props', { folderPath: state.currentProject });
        state.chatHistory = await invoke('get_chat_history', { folderPath: state.currentProject, limit: 20 });
        renderStoryboard();
        renderChatHistory();
    } catch (error) {
        console.error('Failed to load project data:', error);
    }
}

// 渲染分镜表
function renderStoryboard() {
    shotCount.textContent = `${state.storyboards.length} 镜头`;
    storyboardTbody.innerHTML = '';

    if (state.storyboards.length === 0) {
        storyboardTbody.innerHTML = `
            <tr>
                <td colspan="14" class="px-4 py-12 text-center text-slate-400">
                    <p class="mb-2">暂无分镜数据</p>
                    <p class="text-xs">请在右侧 AI 助手对话框中粘贴剧本开始创作</p>
                </td>
            </tr>
        `;
        return;
    }

    state.storyboards.forEach((sb, index) => {
        const row = document.createElement('tr');
        row.className = `hover-row transition-colors group cursor-pointer ${index % 2 === 0 ? 'bg-white' : 'bg-slate-50/40'}`;
        row.innerHTML = `
            <td class="sticky-col-1 px-2 py-2 font-mono text-xs text-slate-400">${sb.sequence_number}</td>
            <td class="sticky-col-2 px-2 py-2 font-bold text-sm text-primary">${sb.mirror_id}</td>
            <td class="px-2 py-2 text-xs">${sb.shot_type || '-'}</td>
            <td class="px-2 py-2 text-xs text-slate-500">${sb.shot_size || '-'}</td>
            <td class="px-2 py-2 text-xs">${sb.duration ? sb.duration + 's' : '-'}</td>
            <td class="px-2 py-2 text-xs italic text-slate-500">${sb.dialogue || '-'}</td>
            <td class="px-2 py-2 text-xs">${sb.description || '-'}</td>
            <td class="px-2 py-2 text-xs text-slate-400">${sb.notes || '-'}</td>
            <td class="px-2 py-2 text-xs">${sb.image_prompt_zh || '-'}</td>
            <td class="px-2 py-2 text-xs font-mono leading-tight">${sb.image_prompt_en || '-'}</td>
            <td class="px-2 py-2 text-xs">${sb.image_prompt_tail_zh || '-'}</td>
            <td class="px-2 py-2 text-xs font-mono leading-tight">${sb.image_prompt_tail_en || '-'}</td>
            <td class="px-2 py-2 text-xs text-primary">${sb.video_prompt_zh || '-'}</td>
            <td class="px-2 py-2 text-xs font-mono leading-tight">${sb.video_prompt_en || '-'}</td>
        `;
        row.addEventListener('click', () => showStoryboardDetail(sb));
        storyboardTbody.appendChild(row);
    });
}

// 切换标签
function switchTab(tabName) {
    state.currentTab = tabName;

    // 更新标签样式
    document.querySelectorAll('[data-tab]').forEach(el => {
        const icon = el.querySelector('.tab-icon');
        const label = el.querySelector('.tab-label');
        if (el.dataset.tab === tabName) {
            icon.className = 'p-2 bg-primary rounded-lg text-white tab-icon';
            label.className = 'vertical-text text-[11px] font-bold text-primary tracking-wide tab-label';
        } else {
            icon.className = 'p-2 text-slate-400 group-hover:bg-slate-100 rounded-lg transition-all tab-icon';
            label.className = 'vertical-text text-[11px] font-bold text-slate-400 tracking-wide tab-label';
        }
    });

    // 显示对应内容
    document.querySelectorAll('.tab-content').forEach(el => el.classList.add('hidden'));
    document.getElementById(`${tabName}-content`).classList.remove('hidden');
}

// 切换布局
function toggleLayout(isThreeCol) {
    const twoColBtn = document.getElementById('two-col-layout');
    const threeColBtn = document.getElementById('three-col-layout');

    if (isThreeCol) {
        threeColBtn.classList.add('active-layout');
        twoColBtn.classList.remove('active-layout');
        aiSidebar.classList.remove('hidden');
    } else {
        twoColBtn.classList.add('active-layout');
        threeColBtn.classList.remove('active-layout');
        aiSidebar.classList.add('hidden');
    }
}

// ========== 分镜详情相关函数 ==========

// 显示分镜详情
function showStoryboardDetail(sb) {
    // 更新标题
    const subtitle = `${sb.mirror_id} · ${sb.shot_size || '-'} · ${sb.shot_type || '-'}镜`;
    document.getElementById('detail-title').textContent = `镜头 #${sb.sequence_number}`;
    document.getElementById('detail-subtitle').textContent = subtitle;

    // 更新内容
    document.getElementById('detail-description').textContent = sb.description || '暂无描述';

    // 台词
    const dialogueSection = document.getElementById('detail-dialogue-section');
    if (sb.dialogue) {
        dialogueSection.classList.remove('hidden');
        document.getElementById('detail-dialogue').textContent = sb.dialogue;
    } else {
        dialogueSection.classList.add('hidden');
    }

    // 图像提示词
    document.getElementById('detail-image-prompt-zh').textContent = sb.image_prompt_zh || '-';
    document.getElementById('detail-image-prompt-en').textContent = sb.image_prompt_en || '-';

    // 视频提示词
    document.getElementById('detail-video-prompt-zh').textContent = sb.video_prompt_zh || '-';
    document.getElementById('detail-video-prompt-en').textContent = sb.video_prompt_en || '-';

    // 备注
    const notesSection = document.getElementById('detail-notes-section');
    if (sb.notes) {
        notesSection.classList.remove('hidden');
        document.getElementById('detail-notes').textContent = sb.notes;
    } else {
        notesSection.classList.add('hidden');
    }

    // 显示弹窗
    storyboardDetailModal.classList.remove('hidden');
}

// 隐藏分镜详情
function hideStoryboardDetail() {
    storyboardDetailModal.classList.add('hidden');
}

// ========== API 配置相关函数 ==========

// 渲染 API 列表
function renderApiList() {
    const apiList = document.getElementById('api-list');
    apiList.innerHTML = '';

    if (state.apis.length === 0) {
        apiList.innerHTML = '<p class="text-center text-slate-400 py-8">暂无 API 配置</p>';
        return;
    }

    state.apis.forEach((api, index) => {
        const div = document.createElement('div');
        div.className = 'flex items-center justify-between p-3 bg-slate-50 rounded-lg border border-slate-200';
        div.innerHTML = `
            <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-lg ${getTypeColor(api.api_type)} flex items-center justify-center">
                    <span class="material-symbols-outlined text-white">${getTypeIcon(api.api_type)}</span>
                </div>
                <div>
                    <div class="font-medium text-sm text-slate-800">${api.name} ${api.is_default ? '<span class="text-xs bg-primary text-white px-1.5 py-0.5 rounded ml-1">默认</span>' : ''}</div>
                    <div class="text-xs text-slate-500">${getTypeLabel(api.api_type)} · ${maskApiKey(api.api_key)}</div>
                </div>
            </div>
            <div class="flex items-center gap-1">
                <button onclick="editApi(${index})" class="p-1.5 hover:bg-slate-200 rounded transition-colors">
                    <span class="material-symbols-outlined text-slate-400 text-[18px]">edit</span>
                </button>
                <button onclick="deleteApi(${index})" class="p-1.5 hover:bg-red-100 rounded transition-colors">
                    <span class="material-symbols-outlined text-red-400 text-[18px]">delete</span>
                </button>
            </div>
        `;
        apiList.appendChild(div);
    });
}

// 获取类型图标
function getTypeIcon(type) {
    const icons = { text: 'chat', image: 'image', video: 'videocam' };
    return icons[type] || 'api';
}

// 获取类型颜色
function getTypeColor(type) {
    const colors = { text: 'bg-blue-500', image: 'bg-green-500', video: 'bg-purple-500' };
    return colors[type] || 'bg-slate-500';
}

// 获取类型标签
function getTypeLabel(type) {
    const labels = { text: '文本生成', image: '图像生成', video: '视频生成' };
    return labels[type] || type;
}

// 掩码 API Key
function maskApiKey(key) {
    if (!key || key.length <= 8) return '****';
    return key.substring(0, 4) + '****' + key.substring(key.length - 4);
}

// 显示 API 编辑弹窗
function showApiEditModal(index) {
    state.editingApiIndex = index;
    const isEdit = index !== null;

    document.getElementById('api-edit-title').textContent = isEdit ? '编辑 API' : '添加 API';

    if (isEdit) {
        const api = state.apis[index];
        document.getElementById('api-name').value = api.name;
        document.getElementById('api-type').value = api.api_type;
        document.getElementById('api-base-url').value = api.base_url;
        document.getElementById('api-key').value = api.api_key;
        document.getElementById('api-model').value = api.model || '';
        document.getElementById('api-default').checked = api.is_default;
    } else {
        document.getElementById('api-name').value = '';
        document.getElementById('api-type').value = 'text';
        document.getElementById('api-base-url').value = '';
        document.getElementById('api-key').value = '';
        document.getElementById('api-model').value = '';
        document.getElementById('api-default').checked = false;
    }

    apiEditModal.classList.remove('hidden');
}

// 隐藏 API 编辑弹窗
function hideApiEditModal() {
    apiEditModal.classList.add('hidden');
    state.editingApiIndex = null;
}

// 确认编辑 API
function handleConfirmApiEdit() {
    const name = document.getElementById('api-name').value.trim();
    const apiType = document.getElementById('api-type').value;
    const baseUrl = document.getElementById('api-base-url').value.trim();
    const apiKey = document.getElementById('api-key').value.trim();
    const model = document.getElementById('api-model').value.trim();
    const isDefault = document.getElementById('api-default').checked;

    if (!name || !baseUrl || !apiKey) {
        alert('请填写完整信息');
        return;
    }

    const apiConfig = {
        id: state.editingApiIndex !== null ? state.apis[state.editingApiIndex].id : Date.now().toString(),
        name,
        api_type: apiType,
        base_url: baseUrl,
        api_key: apiKey,
        model: model || null,
        is_default: isDefault
    };

    // 如果设为默认，清除其他默认
    if (isDefault) {
        state.apis.forEach(api => api.is_default = false);
    }

    if (state.editingApiIndex !== null) {
        state.apis[state.editingApiIndex] = apiConfig;
    } else {
        state.apis.push(apiConfig);
    }

    renderApiList();
    hideApiEditModal();
}

// 编辑 API（全局函数供 HTML 调用）
window.editApi = function(index) {
    showApiEditModal(index);
};

// 删除 API（全局函数供 HTML 调用）
window.deleteApi = function(index) {
    if (confirm('确定要删除这个 API 配置吗？')) {
        state.apis.splice(index, 1);
        renderApiList();
    }
};

// 保存 API 配置
async function handleSaveApiConfig() {
    try {
        await invoke('save_global_config', {
            config: { apis: state.apis }
        });
        hideApiConfigModal();
        // 重新加载配置
        await loadConfig();
    } catch (error) {
        alert('保存配置失败: ' + error);
    }
}

// 显示 API 配置弹窗
function showApiConfigModal() {
    apiConfigModal.classList.remove('hidden');
}

// 隐藏 API 配置弹窗
function hideApiConfigModal() {
    apiConfigModal.classList.add('hidden');
}

// ========== 聊天相关函数 ==========

// 发送聊天消息
async function sendChatMessage() {
    const message = chatInput.value.trim();
    if (!message) return;

    // 检查是否有可用的文本 API
    const textApi = state.apis.find(api => api.api_type === 'text');
    if (!textApi) {
        alert('请先配置一个文本类型的 API');
        showApiConfigModal();
        return;
    }

    // 添加用户消息到界面和状态
    addChatMessage('user', message);
    chatInput.value = '';
    state.chatHistory.push({ role: 'user', content: message });

    // 保存到数据库
    if (state.currentProject) {
        invoke('save_chat_message', {
            folderPath: state.currentProject,
            role: 'user',
            content: message
        }).catch(console.error);
    }

    // 检测操作类型
    const isFullRegenerate = /重做|重新生成|重做一版|重新做|覆盖/.test(message);
    const isPartialUpdate = /插入|新增分镜|拆分|拆开|合并/.test(message);

    // 完全重做：清空所有数据重新生成
    if (isFullRegenerate && state.storyboards.length > 0) {
        const confirmed = confirm('重做将会清空当前所有分镜数据并重新生成，包括您手动修改的内容。\n\n是否确认重做？');
        if (!confirmed) {
            addChatMessage('assistant', '已取消重做。');
            return;
        }
    }

    // 部分更新（插入/拆分/合并/删除）：不清空，只更新变化的部分
    const isRegenerate = isFullRegenerate;

    // 添加加载中的消息
    const loadingDiv = addLoadingMessage();

    try {
        // 构建包含当前分镜列表的上下文信息
        const storyboardContext = state.storyboards.length > 0
            ? `\n\n【当前分镜列表】\n${state.storyboards.map(s => `${s.sequence_number}. ${s.mirror_id}: ${s.description || '-'}`).join('\n')}`
            : '\n\n【当前状态】暂无分镜';

        // 调用后端 API
        const response = await invoke('call_ai_api', {
            apiConfig: textApi,
            message: message + storyboardContext,  // 把当前分镜列表发送给 AI
            chatHistory: state.chatHistory
        });

        // 移除加载消息
        loadingDiv.remove();

        // 尝试解析结构化输出
        const structuredData = parseStructuredResponse(response);

        if (structuredData) {
            // 保存结构化数据到数据库
            await invoke('save_generated_data', {
                folderPath: state.currentProject,
                storyboards: structuredData.storyboards || [],
                characters: structuredData.characters || [],
                scenes: structuredData.scenes || [],
                props: structuredData.props || [],
                isRegenerate: isRegenerate  // 传递是否重做标志
            });

            // 重新加载项目数据
            await loadProjectData();

            // 添加友好的消息提示
            const summary = `已生成 ${structuredData.storyboards?.length || 0} 个分镜`;
            addChatMessage('assistant', summary);
            state.chatHistory.push({ role: 'assistant', content: summary });
        } else {
            // 普通文本响应，直接显示
            addChatMessage('assistant', response);
            state.chatHistory.push({ role: 'assistant', content: response });
        }

        // 保存 AI 响应到数据库
        if (state.currentProject) {
            invoke('save_chat_message', {
                folderPath: state.currentProject,
                role: 'assistant',
                content: response
            }).catch(console.error);
        }
    } catch (error) {
        loadingDiv.remove();
        addChatMessage('assistant', '调用 AI 失败: ' + error);
    }
}

// 处理文件上传
async function handleFileUpload(event) {
    const file = event.target.files[0];
    if (!file) return;

    // 显示文件名消息
    addChatMessage('user', `📎 上传剧本文件: ${file.name}`);

    const fileExt = file.name.split('.').pop().toLowerCase();

    // 处理 Word 文档 (.docx)
    if (fileExt === 'docx') {
        const reader = new FileReader();
        reader.onload = async (e) => {
            try {
                const arrayBuffer = e.target.result;
                const result = await mammoth.extractRawText({ arrayBuffer: arrayBuffer });
                const content = result.value;

                // 将内容填入输入框
                chatInput.value = content;

                addChatMessage('assistant', `已读取 DOCX 文件 (${file.size} 字节)，提取了 ${content.length} 个字符。内容已填入输入框，点击发送按钮开始生成分镜。`);
            } catch (error) {
                console.error('解析 Word 文档失败:', error);
                addChatMessage('assistant', '解析 Word 文档失败，请确保文件格式正确。');
            }
            event.target.value = '';
        };
        reader.onerror = () => {
            addChatMessage('assistant', '读取文件失败，请重试。');
            event.target.value = '';
        };
        reader.readAsArrayBuffer(file);
        return;
    }

    // 处理老格式 Word 文档 (.doc)
    if (fileExt === 'doc') {
        addChatMessage('assistant', '不支持旧版 .doc 格式，请将文件另存为 .docx 格式后再试。');
        event.target.value = '';
        return;
    }

    // 处理文本文件 (.txt, .md, .json)
    const reader = new FileReader();
    reader.onload = async (e) => {
        const content = e.target.result;

        // 将内容填入输入框
        chatInput.value = content;

        // 显示提示消息
        const fileInfo = file.name.match(/\.(txt|md|json)$/i);
        const fileType = fileInfo ? fileInfo[1].toUpperCase() : '文件';
        addChatMessage('assistant', `已读取 ${fileType} 文件 (${file.size} 字节)，内容已填入输入框。点击发送按钮开始生成分镜。`);

        // 清空文件输入
        event.target.value = '';
    };
    reader.onerror = () => {
        addChatMessage('assistant', '读取文件失败，请重试。');
        event.target.value = '';
    };
    reader.readAsText(file);
}

// 添加加载中的消息
function addLoadingMessage() {
    const div = document.createElement('div');
    div.className = 'flex gap-3';
    div.innerHTML = `
        <div class="size-8 rounded-full bg-slate-100 flex items-center justify-center shrink-0 border border-slate-200">
            <span class="material-symbols-outlined text-slate-500 text-[18px]">smart_toy</span>
        </div>
        <div class="flex flex-col gap-1.5 max-w-[85%]">
            <div class="bg-slate-100 text-slate-700 p-3 rounded-2xl rounded-tl-none">
                <div class="flex gap-1">
                    <div class="w-2 h-2 bg-slate-400 rounded-full animate-bounce"></div>
                    <div class="w-2 h-2 bg-slate-400 rounded-full animate-bounce" style="animation-delay: 0.1s"></div>
                    <div class="w-2 h-2 bg-slate-400 rounded-full animate-bounce" style="animation-delay: 0.2s"></div>
                </div>
            </div>
        </div>
    `;
    chatMessages.appendChild(div);
    chatMessages.scrollTop = chatMessages.scrollHeight;
    return div;
}

// 添加聊天消息到界面
function addChatMessage(role, content) {
    const isUser = role === 'user';
    const div = document.createElement('div');
    div.className = `flex gap-3 ${isUser ? 'flex-row-reverse' : ''}`;
    div.innerHTML = `
        <div class="size-8 rounded-full ${isUser ? 'bg-primary text-white' : 'bg-slate-100'} flex items-center justify-center shrink-0 overflow-hidden shadow-sm ${isUser ? 'text-[10px] font-bold' : 'border border-slate-200'}">
            <span class="material-symbols-outlined ${isUser ? '' : 'text-slate-500 text-[18px]'}">${isUser ? 'USER' : 'smart_toy'}</span>
        </div>
        <div class="flex flex-col gap-1.5 ${isUser ? 'items-end' : ''} max-w-[85%]">
            <div class="${isUser ? 'bg-primary text-white rounded-tr-none' : 'bg-slate-100 text-slate-700 rounded-tl-none'} p-3 rounded-2xl ${isUser ? 'shadow-sm' : ''}">
                <p class="text-xs leading-relaxed whitespace-pre-wrap">${escapeHtml(content)}</p>
            </div>
        </div>
    `;
    chatMessages.appendChild(div);
    chatMessages.scrollTop = chatMessages.scrollHeight;
}

// 渲染聊天历史
function renderChatHistory() {
    // 清空当前显示的消息
    chatMessages.innerHTML = '';

    // 如果没有历史记录，显示欢迎消息
    if (state.chatHistory.length === 0) {
        chatMessages.innerHTML = `
            <div class="flex gap-3">
                <div class="size-8 rounded-full bg-slate-100 flex items-center justify-center shrink-0 border border-slate-200">
                    <span class="material-symbols-outlined text-slate-500 text-[18px]">smart_toy</span>
                </div>
                <div class="flex flex-col gap-1.5 max-w-[85%]">
                    <div class="bg-slate-100 text-slate-700 p-3 rounded-2xl rounded-tl-none">
                        <p class="text-xs leading-relaxed">欢迎使用分镜师！请粘贴您的剧本，我将帮您生成分镜表。</p>
                    </div>
                </div>
            </div>
        `;
        // 滚动到底部
        chatMessages.scrollTop = chatMessages.scrollHeight;
        return;
    }

    // 渲染历史消息
    state.chatHistory.forEach(msg => {
        addChatMessage(msg.role, msg.content);
    });

    // 使用 requestAnimationFrame 确保 DOM 渲染完成后再滚动
    requestAnimationFrame(() => {
        chatMessages.scrollTop = chatMessages.scrollHeight;
    });
}

// 转义 HTML
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// 解析结构化响应
function parseStructuredResponse(response) {
    // 尝试匹配 JSON 代码块
    const jsonBlockMatch = response.match(/```(?:json)?\s*(\{[\s\S]*?\})\s*```/);
    if (jsonBlockMatch) {
        try {
            return JSON.parse(jsonBlockMatch[1]);
        } catch (e) {
            console.warn('Failed to parse JSON from code block:', e);
        }
    }

    // 尝试直接解析整个响应为 JSON
    try {
        const parsed = JSON.parse(response.trim());
        // 验证是否包含我们需要的结构
        if (parsed.storyboards || parsed.characters || parsed.scenes || parsed.props) {
            return parsed;
        }
    } catch (e) {
        // 不是纯 JSON 响应，忽略
    }

    // 尝试查找可能的 JSON 对象（包含 storyboards 的）
    const jsonObjectMatch = response.match(/\{[\s\S]*"storyboards"[\s\S]*\}/);
    if (jsonObjectMatch) {
        try {
            return JSON.parse(jsonObjectMatch[0]);
        } catch (e) {
            console.warn('Failed to parse JSON object:', e);
        }
    }

    return null;
}

// 显示主界面
function showMainScreen() {
    welcomeScreen.classList.add('hidden');
    mainScreen.classList.remove('hidden');
    mainScreen.classList.add('flex');
}

// 启动应用
init();
