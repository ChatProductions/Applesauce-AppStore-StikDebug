use crate::objc_classes;
use crate::objc::{id, msg, nil, HostObject, retain, release, autorelease};
use crate::Environment;

// Структура для хранения состояния анимации
pub(super) struct CAKeyframeAnimationHostObject {
    // CAPropertyAnimation properties
    key_path: id,
    
    // CAKeyframeAnimation properties
    values: id,
    path: id,
    key_times: id,
    timing_functions: id,
    calculation_mode: id,
    rotation_mode: id,
    tension_values: id,
    continuity_values: id,
    bias_values: id,
    
    // CAMediaTiming & CAAnimation properties
    duration: f64,
    fill_mode: id,
    delegate: id, // CAAnimation delegate is strongly retained in CoreAnimation!
    removed_on_completion: bool,
}

impl HostObject for CAKeyframeAnimationHostObject {}

pub const CLASSES: crate::objc::ClassExports = objc_classes!
{
    (env, this, _cmd);

    // В оригинальном фреймворке CAKeyframeAnimation наследуется от CAPropertyAnimation,
    // но если в вашей реализации иерархия плоская, оставляем NSObject.
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
            fill_mode: nil,
            delegate: nil,
            removed_on_completion: true,
        });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    + (id)animationWithKeyPath:(id)path {
        let anim: id = msg![env; this alloc];
        let anim: id = msg![env; anim init];
        if path != nil {
            () = msg![env; anim setKeyPath:path];
        }
        autorelease(env, anim)
    }

    - (id)init {
        this
    }

    - (())dealloc {
        // Чтобы избежать ошибки borrow checker, сначала извлекаем все 
        // значения из объекта (освобождая заимствование env), а затем делаем release.
        let (
            key_path, values, path, key_times, timing_functions,
            calculation_mode, rotation_mode, tension_values,
            continuity_values, bias_values, fill_mode, delegate
        ) = {
            let host = env.objc.borrow::<CAKeyframeAnimationHostObject>(this);
            (
                host.key_path, host.values, host.path, host.key_times, host.timing_functions,
                host.calculation_mode, host.rotation_mode, host.tension_values,
                host.continuity_values, host.bias_values, host.fill_mode, host.delegate
            )
        };
        
        // Освобождаем все retained объекты (включая delegate)
        if key_path != nil { release(env, key_path); }
        if values != nil { release(env, values); }
        if path != nil { release(env, path); }
        if key_times != nil { release(env, key_times); }
        if timing_functions != nil { release(env, timing_functions); }
        if calculation_mode != nil { release(env, calculation_mode); }
        if rotation_mode != nil { release(env, rotation_mode); }
        if tension_values != nil { release(env, tension_values); }
        if continuity_values != nil { release(env, continuity_values); }
        if bias_values != nil { release(env, bias_values); }
        if fill_mode != nil { release(env, fill_mode); }
        if delegate != nil { release(env, delegate); }
        
        env.objc.dealloc_object(this, &mut env.mem)
    }

    // --- Геттеры и сеттеры ---

    - (id)keyPath { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path }
    - (())setKeyPath:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_path = val;
        if old != nil { release(env, old); }
    }

    - (id)values { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values }
    - (())setValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).values = val;
        if old != nil { release(env, old); }
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
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_times = val;
        if old != nil { release(env, old); }
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
    - (())setDuration:(f64)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).duration = val; }

    - (id)fillMode { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).fill_mode }
    - (())setFillMode:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).fill_mode;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).fill_mode = val;
        if old != nil { release(env, old); }
    }

    - (id)delegate { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).delegate }
    - (())setDelegate:(id)val {
        // Delegate в CoreAnimation является strong property! 
        // Поэтому здесь обязательно нужно делать retain/release.
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).delegate;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).delegate = val;
        if old != nil { release(env, old); }
    }

    - (bool)removedOnCompletion { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).removed_on_completion }
    - (())setRemovedOnCompletion:(bool)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).removed_on_completion = val; }

    @end
};
