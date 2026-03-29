use super::*;

// ---------------------------------------------------------------------------
// Element API tests
// ---------------------------------------------------------------------------

#[test]
fn element_children_returns_elements_only() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.appendChild(document.createElement('span'));
        div.appendChild(document.createTextNode('text'));
        div.appendChild(document.createElement('p'));
        console.log(div.children.length === 2);
    ",
    );
}

#[test]
fn element_outer_html() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.setAttribute('class', 'test');
        var html = div.outerHTML;
        console.log(html.indexOf('<div') === 0 && html.indexOf('class') > 0);
    ",
    );
}

#[test]
fn classlist_length() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.classList.add('a');
        div.classList.add('b');
        div.classList.add('c');
        console.log(div.classList.length === 3);
    ",
    );
}

#[test]
fn classlist_item() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        div.classList.add('first');
        div.classList.add('second');
        console.log(div.classList.item(0) === 'first');
    ",
    );
}

// ---------------------------------------------------------------------------
// Element.animate
// ---------------------------------------------------------------------------

#[test]
fn element_animate_returns_animation() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        var anim = div.animate(
            [{ opacity: 0 }, { opacity: 1 }],
            { duration: 1000, iterations: 1 }
        );
        console.log(anim.playState === 'running' && typeof anim.play === 'function');
    ",
    );
}

#[test]
fn element_animate_cancel() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        var anim = div.animate([{ opacity: 0 }, { opacity: 1 }], 500);
        anim.cancel();
        console.log(anim.playState === 'idle');
    ",
    );
}

#[test]
fn element_animate_composite_option() {
    let (mut rt, mut s, mut d, doc) = setup();
    eval_true(
        &mut rt,
        &mut s,
        &mut d,
        doc,
        r"
        var div = document.createElement('div');
        var anim = div.animate(
            [{ opacity: 0 }, { opacity: 1 }],
            { duration: 1000, composite: 'add' }
        );
        console.log(anim.playState === 'running');
    ",
    );
}
