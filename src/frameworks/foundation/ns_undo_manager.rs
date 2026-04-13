use crate::objc_classes;
use crate::objc::{id, SEL};
use crate::Environment;
use std::collections::HashMap;

// Структура для хранения одного шага отмены (кто, что и с какими аргументами должен сделать)
pub struct UndoAction {
    target: id,
    selector: SEL,
    object: id,
}

// Состояние модуля: храним стек действий для каждого созданного экземпляра NSUndoManager
#[derive(Default)]
pub struct State {
    stacks: HashMap<id, Vec<UndoAction>>,
}

pub const CLASSES: crate::objc::ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation NSUndoManager : NSObject

    - (id) init {
        // Инициализируем пустой стек действий для этого конкретного менеджера
        env.foundation.ns_undo_manager.stacks.insert(this, Vec::new());
        this
    }

    - (void) dealloc {
        // Очищаем память (удаляем стек), когда менеджер больше не нужен
        env.foundation.ns_undo_manager.stacks.remove(&this);
    }

    - (void) registerUndoWithTarget:(id)target selector:(SEL)selector object:(id)anObject {
        // Сохраняем действие в стек
        if let Some(stack) = env.foundation.ns_undo_manager.stacks.get_mut(&this) {
            stack.push(UndoAction { target, selector, object: anObject });
            crate::log!("NSUndoManager: Зарегистрировано действие. Всего в стеке: {}", stack.len());
        }
    }

    - (void) removeAllActions {
        // Очищаем все сохраненные действия
        if let Some(stack) = env.foundation.ns_undo_manager.stacks.get_mut(&this) {
            stack.clear();
        }
    }

    - (bool) canUndo {
        // Возвращаем true, если в стеке есть хотя бы одно действие
        env.foundation.ns_undo_manager.stacks.get(&this).map_or(false, |s| !s.is_empty())
    }

    - (void) undo {
        // Извлекаем последнее действие из стека и выполняем его
        if let Some(stack) = env.foundation.ns_undo_manager.stacks.get_mut(&this) {
            if let Some(action) = stack.pop() {
                crate::log!("NSUndoManager: Выполняем undo!");
                
                // Вызываем метод у целевого объекта через внутренний механизм отправки сообщений эмулятора.
                // anObject передается как первый аргумент (в u32).
                env.objc.msg_send(
                    env, 
                    action.target, 
                    action.selector, 
                    &[action.object as u32] 
                );
            }
        }
    }

    @end
};
