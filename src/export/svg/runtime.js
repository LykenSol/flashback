(function() {
    function int(x) {
        return x | 0;
    }

    function svg_element(tag) {
        return document.createElementNS('http://www.w3.org/2000/svg', tag);
    }

    var rt = {};
    rt.mkGlobalScope = function() {
        var o = Object.create(null);
        function def(name, x) {
            Object.defineProperty(o, name, { value: x });
        }
        def('hasOwnProperty', function(o, x) {
            return Object.prototype.hasOwnProperty.call(o, x);
        });
        // HACK(eddyb) trap writes.
        if(Object.freeze)
            return Object.freeze(o);
        return o;
    };
    rt.mkLocalScope = function(_this) {
        var o = Object.create(_this);
        function def(name, x) {
            Object.defineProperty(o, name, { value: x });
        }
        def('this', _this);
        // HACK(eddyb) trap writes.
        if(Object.freeze)
            return Object.freeze(o);
        return o;
    };
    rt.mkMovieClip = function(timeline) {
        var o = Object.create(null);
        function def_get(name, f) {
            Object.defineProperty(o, name, { get: f });
        }
        function def(name, x) {
            Object.defineProperty(o, name, { value: x });
        }
        def('play', function() {
            timeline.paused = false;
        });
        def('stop', function() {
            timeline.paused = true;
        });
        def('gotoAndPlay', function(frame) {
            timeline.frame = frame;
            timeline.paused = false;
        });
        if(timeline.parent)
            def_get('_parent', rt.mkMovieClip.bind(null, timeline.parent));
        for(var name in timeline.named) {
            var layer = timeline.layers[timeline.named[name]];
            if(layer && layer.sprite)
                def_get(name, rt.mkMovieClip.bind(null, layer.sprite));
        }
        // HACK(eddyb) trap writes.
        if(Object.freeze)
            return Object.freeze(o);
        return api;
    };

    function Timeline(data, container) {
        if(!(this instanceof Timeline))
            return new Timeline(data);

        this.frame_count = data.frame_count;
        this.named = Object.create(null);
        this.actions = data.actions;
        this.layers = data.layers.map(function(frames, i) {
            var container = svg_element('g');
            var use = svg_element('use');
            container.appendChild(use);
            return { frames: frames, container: container, use: use };
        });
        this.container = container;
        this.attachLayers();
    }
    Timeline.prototype.paused = false;
    Timeline.prototype.frame = 0;
    Timeline.prototype.attachLayers = function() {
        var container = this.container;
        this.layers.forEach(function(layer) {
            container.appendChild(layer.container);
        });
    };
    Timeline.prototype.detachLayers = function() {
        this.layers.forEach(function(layer) {
            layer.container.remove();
        });
    };
    Timeline.prototype.showFrame = function() {
        if(this.paused) {
            // Update sprites even when paused.
            this.layers.forEach(function(layer) {
                if(layer.sprite)
                    layer.sprite.showFrame();
            });
            return;
        }

        var frame = this.frame;
        var named = this.named;
        this.layers.forEach(function(layer, depth) {
            var obj = layer.frames[frame];

            // TODO(eddyb) this might need to take SWF's `is_move` into account.
            // Remove the old character if necessary.
            if(obj === null || (obj && layer.character != obj.character)) {
                layer.character = -1;
                layer.use.removeAttribute('href');
                if(layer.sprite) {
                    layer.sprite.detachLayers();
                    layer.sprite.parent = null;
                    layer.sprite = null;
                }
            }

            // Remove the old name if necessary.
            if(obj === null || (obj && layer.name != obj.name)) {
                named[layer.name] = null;
                layer.name = null;
            }

            if(obj) {
                if(layer.character != obj.character) {
                    layer.character = obj.character;
                    layer.use.setAttribute('href', '#c_' + obj.character);

                    var sprite_data = sprites[obj.character];
                    if(sprite_data) {
                        layer.sprite = new Timeline(sprite_data, layer.container);
                        layer.sprite.parent = this;
                    }
                }
                layer.container.setAttribute('transform', 'matrix(' + obj.matrix.join(' ') + ')');
                if(layer.name != obj.name) {
                    layer.name = obj.name;
                    named[layer.name] = depth;
                }
            }

            // Update the sprite if it exists.
            if(layer.sprite)
                layer.sprite.showFrame();
        });

        var action = this.actions[frame];
        if(action)
            action(rt.mkGlobalScope(), rt.mkLocalScope(rt.mkMovieClip(this)));

        // HACK(eddyb) no idea what the interaction here should be.
        if(!this.paused)
            this.frame = (frame + 1) % this.frame_count;
    };

    timeline = new Timeline(timeline, document.getElementById('body'));

    var start;
    var last_frame = 0;
    function update(now) {
        window.requestAnimationFrame(update);

        if(!start) start = now;
        // TODO(eddyb) figure out how to avoid absolute values.
        var frame = int((now - start) * frame_rate / 1000);

        for(; last_frame < frame; last_frame++)
            timeline.showFrame();
    }
    window.requestAnimationFrame(update);
})()
