class TodoApp {
    constructor() {
        this.basePath = window.BASE_PATH || '';
        this.todos = [];
        this.tokens = [];
        this.draggedItem = null;
        this.editingId = null;
        // Touch drag state
        this.touchDragId = null;
        this.touchClone = null;
        this.touchStartY = 0;
        this.touchCurrentX = 0;
        this.touchCurrentY = 0;
        // Bound handlers for touch events (needed for removeEventListener)
        this.boundTouchMove = this.handleTouchMove.bind(this);
        this.boundTouchEnd = this.handleTouchEnd.bind(this);
        this.init();
    }

    init() {
        this.bindElements();
        this.bindEvents();
        this.loadTodos();
    }

    bindElements() {
        this.addTodoForm = document.getElementById('add-todo-form');
        this.createTokenForm = document.getElementById('create-token-form');

        this.todoTitle = document.getElementById('todo-title');
        this.tokenName = document.getElementById('token-name');

        this.todosList = document.getElementById('todos-list');
        this.tokensList = document.getElementById('tokens-list');

        this.completedSection = document.getElementById('completed-section');
        this.completedList = document.getElementById('completed-list');
        this.toggleCompletedBtn = document.getElementById('toggle-completed');
        this.completedChevron = document.getElementById('completed-chevron');
        this.completedCount = document.getElementById('completed-count');

        this.completedExpanded = false;

        this.logoutBtn = document.getElementById('logout-btn');
        this.tokensBtn = document.getElementById('tokens-btn');
        this.closeTokensModal = document.getElementById('close-tokens-modal');

        this.tokensModal = document.getElementById('tokens-modal');
    }

    bindEvents() {
        this.addTodoForm.addEventListener('submit', (e) => this.handleAddTodo(e));
        this.createTokenForm.addEventListener('submit', (e) => this.handleCreateToken(e));

        this.logoutBtn.addEventListener('click', () => this.handleLogout());
        this.tokensBtn.addEventListener('click', () => this.openTokensModal());
        this.closeTokensModal.addEventListener('click', () => this.closeModal(this.tokensModal));

        this.tokensModal.addEventListener('click', (e) => {
            if (e.target === this.tokensModal) this.closeModal(this.tokensModal);
        });

        this.toggleCompletedBtn.addEventListener('click', () => this.toggleCompletedSection());

        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeModal(this.tokensModal);
            }
        });
    }

    async loadTodos() {
        try {
            const response = await fetch(`${this.basePath}/api/todos`);
            if (response.status === 401) {
                window.location.href = `${this.basePath}/login`;
                return;
            }
            this.todos = await response.json();
            this.renderTodos();
        } catch (error) {
            console.error('Failed to load todos:', error);
        }
    }

    toggleCompletedSection() {
        this.completedExpanded = !this.completedExpanded;
        this.completedList.classList.toggle('hidden', !this.completedExpanded);
        this.completedChevron.classList.toggle('rotate-90', this.completedExpanded);
    }

    renderTodos() {
        const openTodos = this.todos.filter(t => !t.completed);
        const completedTodos = this.todos.filter(t => t.completed);

        // Render open todos
        this.todosList.innerHTML = openTodos.map(todo => this.renderTodoItem(todo)).join('');

        // Render completed section
        if (completedTodos.length > 0) {
            this.completedSection.classList.remove('hidden');
            this.completedCount.textContent = `Erledigt (${completedTodos.length})`;
            this.completedList.innerHTML = completedTodos.map(todo => this.renderTodoItem(todo)).join('');
        } else {
            this.completedSection.classList.add('hidden');
        }

        this.todos.forEach(todo => {
            const item = document.getElementById(`todo-${todo.id}`);
            const checkbox = document.getElementById(`checkbox-${todo.id}`);

            checkbox.addEventListener('change', () => this.toggleTodo(todo.id));

            if (todo.completed) {
                const deleteBtn = document.getElementById(`delete-${todo.id}`);
                if (deleteBtn) {
                    deleteBtn.addEventListener('click', () => this.deleteTodo(todo.id));
                }
            } else {
                const dragHandle = document.getElementById(`drag-${todo.id}`);

                if (this.editingId === todo.id) {
                    const input = document.getElementById(`edit-input-${todo.id}`);
                    input.focus();
                    input.select();
                } else {
                    const titleEl = document.getElementById(`title-${todo.id}`);
                    titleEl.addEventListener('click', () => this.startEdit(todo.id));
                }

                dragHandle.addEventListener('mousedown', () => item.draggable = true);
                dragHandle.addEventListener('mouseup', () => item.draggable = false);

                // Touch events for mobile drag-and-drop
                dragHandle.addEventListener('touchstart', (e) => this.handleTouchStart(e, todo.id), { passive: false });

                item.addEventListener('dragstart', (e) => this.handleDragStart(e, todo.id));
                item.addEventListener('dragend', (e) => this.handleDragEnd(e));
                item.addEventListener('dragover', (e) => this.handleDragOver(e));
                item.addEventListener('dragenter', (e) => this.handleDragEnter(e));
                item.addEventListener('dragleave', (e) => this.handleDragLeave(e));
                item.addEventListener('drop', (e) => this.handleDrop(e, todo.id));
            }
        });

        // Bind edit input events after render
        if (this.editingId) {
            const input = document.getElementById(`edit-input-${this.editingId}`);
            if (input) {
                input.addEventListener('blur', (e) => this.handleEditBlur(e));
                input.addEventListener('keydown', (e) => this.handleEditKeydown(e));
            }
        }
    }

    renderTodoItem(todo) {
        const isEditing = this.editingId === todo.id;
        const isCompleted = todo.completed;

        return `
            <div id="todo-${todo.id}" data-id="${todo.id}" class="todo-item bg-white dark:bg-gray-800 rounded-lg shadow-sm p-4 flex items-center gap-3 border border-gray-200 dark:border-gray-700 hover:shadow-md transition-all duration-300">
                ${!isCompleted ? `
                    <div
                        id="drag-${todo.id}"
                        class="cursor-grab text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 p-1"
                        title="Drag to reorder"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8h16M4 16h16"></path>
                        </svg>
                    </div>
                ` : ''}
                <input
                    type="checkbox"
                    id="checkbox-${todo.id}"
                    class="h-5 w-5 rounded border-gray-300 dark:border-gray-600 text-green-600 focus:ring-green-500 cursor-pointer"
                    ${isCompleted ? 'checked' : ''}
                    title="${isCompleted ? 'Mark as not done' : 'Mark as done'}"
                >
                ${isEditing ? `
                    <input
                        type="text"
                        id="edit-input-${todo.id}"
                        value="${this.escapeAttr(todo.title)}"
                        class="flex-1 min-w-0 px-2 py-1 border border-blue-400 dark:bg-gray-700 dark:text-gray-100 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
                    >
                ` : `
                    <span
                        id="title-${todo.id}"
                        class="flex-1 min-w-0 font-medium ${isCompleted ? 'text-gray-500 dark:text-gray-400 line-through' : 'text-gray-800 dark:text-gray-100 cursor-text hover:text-blue-600 dark:hover:text-blue-400'}"
                        title="${isCompleted ? '' : 'Click to edit'}"
                    >${this.escapeHtml(todo.title)}</span>
                `}
                ${isCompleted ? `
                    <button
                        id="delete-${todo.id}"
                        class="flex-shrink-0 p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
                        title="Delete"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                        </svg>
                    </button>
                ` : ''}
            </div>
        `;
    }

    startEdit(id) {
        if (this.editingId === id) return;
        this.editingId = id;
        this.renderTodos();
    }

    handleEditKeydown(e) {
        if (e.key === 'Enter') {
            e.preventDefault();
            this.saveEdit();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            this.cancelEdit();
        }
    }

    handleEditBlur(e) {
        // Small delay to allow click events to fire first
        setTimeout(() => {
            if (this.editingId) {
                this.saveEdit();
            }
        }, 100);
    }

    async saveEdit() {
        if (!this.editingId) return;

        const id = this.editingId;
        const input = document.getElementById(`edit-input-${id}`);
        if (!input) {
            this.editingId = null;
            this.renderTodos();
            return;
        }

        const newTitle = input.value.trim();
        const todo = this.todos.find(t => t.id === id);

        this.editingId = null;

        if (!newTitle || newTitle === todo.title) {
            this.renderTodos();
            return;
        }

        try {
            const response = await fetch(`${this.basePath}/api/todos/${id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ title: newTitle }),
            });

            if (response.ok) {
                const updated = await response.json();
                const index = this.todos.findIndex(t => t.id === id);
                if (index !== -1) {
                    this.todos[index] = updated;
                }
            }
        } catch (error) {
            console.error('Failed to update todo:', error);
        }

        this.renderTodos();
    }

    cancelEdit() {
        this.editingId = null;
        this.renderTodos();
    }

    async toggleTodo(id) {
        const todo = this.todos.find(t => t.id === id);
        if (!todo) return;

        const newCompleted = !todo.completed;

        try {
            const response = await fetch(`${this.basePath}/api/todos/${id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ completed: newCompleted }),
            });

            if (response.ok) {
                const updated = await response.json();
                const index = this.todos.findIndex(t => t.id === id);
                if (index !== -1) {
                    this.todos[index] = updated;
                }
                this.renderTodos();
            }
        } catch (error) {
            console.error('Failed to toggle todo:', error);
        }
    }

    async deleteTodo(id) {
        const item = document.getElementById(`todo-${id}`);
        if (item) {
            item.classList.add('completing');
            item.style.transform = 'translateX(100%)';
            item.style.opacity = '0';
        }

        try {
            await fetch(`${this.basePath}/api/todos/${id}`, {
                method: 'DELETE',
            });

            setTimeout(() => {
                this.todos = this.todos.filter(t => t.id !== id);
                this.renderTodos();
            }, 300);
        } catch (error) {
            console.error('Failed to delete todo:', error);
            if (item) {
                item.style.transform = '';
                item.style.opacity = '';
            }
        }
    }

    handleDragStart(e, id) {
        this.draggedItem = id;
        e.target.classList.add('opacity-50');
        e.dataTransfer.effectAllowed = 'move';
    }

    handleDragEnd(e) {
        e.target.classList.remove('opacity-50');
        e.target.draggable = false;
        document.querySelectorAll('.drag-over').forEach(el => el.classList.remove('drag-over', 'border-blue-400', 'border-2'));
    }

    handleDragOver(e) {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'move';
    }

    handleDragEnter(e) {
        e.preventDefault();
        const item = e.target.closest('[data-id]');
        if (item) item.classList.add('drag-over', 'border-blue-400', 'border-2');
    }

    handleDragLeave(e) {
        const item = e.target.closest('[data-id]');
        if (item && !item.contains(e.relatedTarget)) {
            item.classList.remove('drag-over', 'border-blue-400', 'border-2');
        }
    }

    async handleDrop(e, targetId) {
        e.preventDefault();
        const item = e.target.closest('[data-id]');
        if (item) item.classList.remove('drag-over', 'border-blue-400', 'border-2');

        if (this.draggedItem === targetId) return;

        const draggedIndex = this.todos.findIndex(t => t.id === this.draggedItem);
        const targetIndex = this.todos.findIndex(t => t.id === targetId);

        if (draggedIndex === -1 || targetIndex === -1) return;

        const [removed] = this.todos.splice(draggedIndex, 1);
        this.todos.splice(targetIndex, 0, removed);

        this.renderTodos();
        await this.saveOrder();
    }

    // Touch drag-and-drop for mobile
    handleTouchStart(e, id) {
        e.preventDefault();

        const touch = e.touches[0];
        const item = document.getElementById(`todo-${id}`);
        if (!item) return;

        this.touchDragId = id;
        this.touchStartY = touch.clientY;
        this.touchCurrentX = touch.clientX;
        this.touchCurrentY = touch.clientY;

        // Create a visual clone
        const rect = item.getBoundingClientRect();
        this.touchClone = item.cloneNode(true);
        this.touchClone.id = 'touch-drag-clone';
        this.touchClone.style.position = 'fixed';
        this.touchClone.style.left = `${rect.left}px`;
        this.touchClone.style.top = `${rect.top}px`;
        this.touchClone.style.width = `${rect.width}px`;
        this.touchClone.style.zIndex = '1000';
        this.touchClone.style.opacity = '0.9';
        this.touchClone.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
        this.touchClone.style.pointerEvents = 'none';
        document.body.appendChild(this.touchClone);

        // Dim the original
        item.style.opacity = '0.3';

        // Add move and end listeners
        document.addEventListener('touchmove', this.boundTouchMove, { passive: false });
        document.addEventListener('touchend', this.boundTouchEnd);
        document.addEventListener('touchcancel', this.boundTouchEnd);
    }

    handleTouchMove(e) {
        if (!this.touchDragId || !this.touchClone) return;
        e.preventDefault();

        const touch = e.touches[0];
        this.touchCurrentX = touch.clientX;
        this.touchCurrentY = touch.clientY;
        const deltaY = this.touchCurrentY - this.touchStartY;

        // Move the clone
        const item = document.getElementById(`todo-${this.touchDragId}`);
        if (item) {
            const rect = item.getBoundingClientRect();
            this.touchClone.style.top = `${rect.top + deltaY}px`;
        }

        // Highlight drop target
        this.updateTouchDropTarget(touch.clientX, touch.clientY);
    }

    updateTouchDropTarget(x, y) {
        // Remove previous highlights
        document.querySelectorAll('.drag-over').forEach(el => {
            el.classList.remove('drag-over', 'border-blue-400', 'border-2');
        });

        // Find element under touch point (excluding clone)
        if (this.touchClone) this.touchClone.style.display = 'none';
        const elementBelow = document.elementFromPoint(x, y);
        if (this.touchClone) this.touchClone.style.display = '';

        if (elementBelow) {
            const targetItem = elementBelow.closest('[data-id]');
            if (targetItem && targetItem.dataset.id != this.touchDragId) {
                targetItem.classList.add('drag-over', 'border-blue-400', 'border-2');
            }
        }
    }

    async handleTouchEnd(e) {
        if (!this.touchDragId) return;

        // Remove listeners
        document.removeEventListener('touchmove', this.boundTouchMove);
        document.removeEventListener('touchend', this.boundTouchEnd);
        document.removeEventListener('touchcancel', this.boundTouchEnd);

        // Find drop target
        const draggedId = this.touchDragId;
        let targetId = null;

        // Hide clone to find element below
        if (this.touchClone) this.touchClone.style.display = 'none';
        const elementBelow = document.elementFromPoint(
            this.touchCurrentX,
            this.touchCurrentY
        );

        if (elementBelow) {
            const targetItem = elementBelow.closest('[data-id]');
            if (targetItem && targetItem.dataset.id != draggedId) {
                targetId = parseInt(targetItem.dataset.id);
            }
        }

        // Clean up
        if (this.touchClone) {
            this.touchClone.remove();
            this.touchClone = null;
        }

        const item = document.getElementById(`todo-${draggedId}`);
        if (item) item.style.opacity = '';

        document.querySelectorAll('.drag-over').forEach(el => {
            el.classList.remove('drag-over', 'border-blue-400', 'border-2');
        });

        // Perform reorder if we have a valid target
        if (targetId !== null) {
            const draggedIndex = this.todos.findIndex(t => t.id === draggedId);
            const targetIndex = this.todos.findIndex(t => t.id === targetId);

            if (draggedIndex !== -1 && targetIndex !== -1) {
                const [removed] = this.todos.splice(draggedIndex, 1);
                this.todos.splice(targetIndex, 0, removed);
                this.renderTodos();
                await this.saveOrder();
            }
        }

        this.touchDragId = null;
        this.touchStartY = 0;
        this.touchCurrentX = 0;
        this.touchCurrentY = 0;
    }

    async saveOrder() {
        try {
            const ids = this.todos.map(t => t.id);
            await fetch(`${this.basePath}/api/todos/reorder`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ ids }),
            });
        } catch (error) {
            console.error('Failed to save order:', error);
        }
    }

    async handleAddTodo(e) {
        e.preventDefault();

        const title = this.todoTitle.value.trim();
        if (!title) return;

        try {
            const response = await fetch(`${this.basePath}/api/todos`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ title }),
            });

            if (response.ok) {
                const todo = await response.json();
                this.todos.push(todo);
                this.renderTodos();
                this.addTodoForm.reset();

                const item = document.getElementById(`todo-${todo.id}`);
                if (item) {
                    item.style.opacity = '0';
                    item.style.transform = 'translateY(-10px)';
                    requestAnimationFrame(() => {
                        item.style.transition = 'all 0.3s ease';
                        item.style.opacity = '1';
                        item.style.transform = 'translateY(0)';
                    });
                }
            }
        } catch (error) {
            console.error('Failed to add todo:', error);
        }
    }

    async handleLogout() {
        try {
            await fetch(`${this.basePath}/api/logout`, { method: 'POST' });
            window.location.href = `${this.basePath}/login`;
        } catch (error) {
            console.error('Failed to logout:', error);
        }
    }

    async openTokensModal() {
        this.tokensModal.classList.remove('hidden');
        await this.loadTokens();
    }

    async loadTokens() {
        try {
            const response = await fetch(`${this.basePath}/api/tokens`);
            if (response.ok) {
                this.tokens = await response.json();
                this.renderTokens();
            }
        } catch (error) {
            console.error('Failed to load tokens:', error);
        }
    }

    renderTokens() {
        if (this.tokens.length === 0) {
            this.tokensList.innerHTML = '<p class="text-gray-500 dark:text-gray-400 text-center">No API tokens yet.</p>';
            return;
        }

        this.tokensList.innerHTML = this.tokens.map(token => `
            <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700 rounded-md mb-2">
                <div class="flex-1 min-w-0">
                    <p class="font-medium text-gray-800 dark:text-gray-100">${this.escapeHtml(token.name || 'Unnamed token')}</p>
                    <p class="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">${token.token}</p>
                </div>
                <button
                    onclick="app.revokeToken(${token.id})"
                    class="ml-2 p-2 text-red-600 hover:bg-red-100 rounded transition-colors"
                    title="Revoke token"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                </button>
            </div>
        `).join('');
    }

    async handleCreateToken(e) {
        e.preventDefault();

        const name = this.tokenName.value.trim() || null;

        try {
            const response = await fetch(`${this.basePath}/api/tokens`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name }),
            });

            if (response.ok) {
                const token = await response.json();
                this.tokens.unshift(token);
                this.renderTokens();
                this.createTokenForm.reset();
                alert(`New token created!\n\nToken: ${token.token}\n\nMake sure to copy it now - you won't be able to see the full token again.`);
            }
        } catch (error) {
            console.error('Failed to create token:', error);
        }
    }

    async revokeToken(id) {
        if (!confirm('Are you sure you want to revoke this token?')) return;

        try {
            const response = await fetch(`${this.basePath}/api/tokens/${id}`, {
                method: 'DELETE',
            });

            if (response.ok) {
                this.tokens = this.tokens.filter(t => t.id !== id);
                this.renderTokens();
            }
        } catch (error) {
            console.error('Failed to revoke token:', error);
        }
    }

    closeModal(modal) {
        modal.classList.add('hidden');
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    escapeAttr(text) {
        return text.replace(/"/g, '&quot;').replace(/'/g, '&#39;');
    }
}

const app = new TodoApp();
