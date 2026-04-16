use crate::objc_classes;
use crate::objc::{id, msg, nil, HostObject, retain, release, autorelease};
use crate::Environment;

// Структура для хранения полного состояния CAKeyframeAnimation согласно спецификации Apple
pub(super) struct CAKeyframeAnimationHostObject {
    // Свойства CAPropertyAnimation
    key_path: id,
    
    // Свойства предоставления значений
    values: id,
    path: id, // CGPathRef
    
    // Синхронизация
    key_times: id,
    timing_functions: id,
    calculation_mode: id,
    
    // Атрибуты вращения и кубической моды
    rotation_mode: id,
    tension_values: id,
    continuity_values: id,
    bias_values: id,
    
    // Свойства CAAnimation
    duration: f64,
    delegate: id, // Обычно weak
    removed_on_completion: bool,
}
impl HostObject for CAKeyframeAnimationHostObject {}

pub const CLASSES: crate::objc::ClassExports = objc_classes!
{
    (env, this, _cmd);

    @implementation CAKeyframeAnimation : NSObject

    + (id)alloc {
        let host_object = Box::new(CAKeyframeAnimationHostObject {
            key_path: nil,
            values: nil,
            path: nil,
            key_times: nil,
            timing_functions: nil,
            calculation_mode: nil,
            rotation_mode: nil,
            tension_values: nil,
            continuity_values: nil,
            bias_values: nil,
            duration: 0.0,
            delegate: nil,
            removed_on_completion: true,
        });
        [span_1](start_span)env.objc.alloc_object(this, host_object, &mut env.mem)[span_1](end_span)
    }

    + (id)animationWithKeyPath:(id)path {
        [span_2](start_span)let anim: id = msg![env; this alloc];[span_2](end_span)
        let anim: id = msg![env; anim init];
        if path != nil {
            () [span_3](start_span)= msg![env; anim setKeyPath:path];[span_3](end_span)
        }
        autorelease(env, anim)
    }

    - (id)init {
        this
    }

    - (())dealloc {
        let host = env.objc.borrow::<CAKeyframeAnimationHostObject>(this);
        
        [span_4](start_span)// Освобождаем все удерживаемые объекты[span_4](end_span)
        if host.key_path != nil { release(env, host.key_path); }
        if host.values != nil { release(env, host.values); }
        if host.path != nil { release(env, host.path); }
        if host.key_times != nil { release(env, host.key_times); }
        if host.timing_functions != nil { release(env, host.timing_functions); }
        if host.calculation_mode != nil { release(env, host.calculation_mode); }
        if host.rotation_mode != nil { release(env, host.rotation_mode); }
        if host.tension_values != nil { release(env, host.tension_values); }
        if host.continuity_values != nil { release(env, host.continuity_values); }
        if host.bias_values != nil { release(env, host.bias_values); }
        
        env.objc.dealloc_object(this, &mut env.mem)
    }

    // --- Геттеры и сеттеры ---

    - (id)keyPath { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path }
    - (())setKeyPath:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path;
        if val != nil { retain(env, val); [span_5](start_span)}
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_path = val;
        if old != nil { release(env, old); }[span_5](end_span)
    }

    - (id)values { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values }
    - (())setValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values;
        if val != nil { retain(env, val); [span_6](start_span)}
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).values = val;
        if old != nil { release(env, old); }[span_6](end_span)
    }

    - (id)path { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).path }
    - (())setPath:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).path;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).path = val;
        if old != nil { release(env, old); }
    }

    - (id)keyTimes { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_times }
    - (())setKeyTimes:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_times;
        if val != nil { retain(env, val); [span_7](start_span)}
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_times = val;
        if old != nil { release(env, old); }[span_7](end_span)
    }

    - (id)timingFunctions { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).timing_functions }
    - (())setTimingFunctions:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).timing_functions;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).timing_functions = val;
        if old != nil { release(env, old); }
    }

    - (id)calculationMode { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).calculation_mode }
    - (())setCalculationMode:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).calculation_mode;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).calculation_mode = val;
        if old != nil { release(env, old); }
    }

    - (id)rotationMode { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).rotation_mode }
    - (())setRotationMode:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).rotation_mode;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).rotation_mode = val;
        if old != nil { release(env, old); }
    }

    - (id)tensionValues { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).tension_values }
    - (())setTensionValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).tension_values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).tension_values = val;
        if old != nil { release(env, old); }
    }

    - (id)continuityValues { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).continuity_values }
    - (())setContinuityValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).continuity_values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).continuity_values = val;
        if old != nil { release(env, old); }
    }

    - (id)biasValues { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).bias_values }
    - (())setBiasValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).bias_values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).bias_values = val;
        if old != nil { release(env, old); }
    }

    - (f64)duration { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).duration }
    - (())setDuration:(f64)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).duration = val; [span_8](start_span)}

    - (id)delegate { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).delegate }
    - (())setDelegate:(id)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).delegate = val; }[span_8](end_span)

    - (bool)removedOnCompletion { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).removed_on_completion }
    - (())setRemovedOnCompletion:(bool)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).removed_on_completion = val; [span_9](start_span)}

    @end
};

