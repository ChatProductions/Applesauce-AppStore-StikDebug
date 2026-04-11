/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITableView`, `UITableViewCell`.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{id, msg_super, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITableViewCell: UIView

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

- (id)initWithStyle:(i32)_style reuseIdentifier:(id)_identifier {
    msg_super![env; this init]
}

- (id)reuseIdentifier { nil }
- (id)textLabel { nil }
- (id)detailTextLabel { nil }
- (id)imageView { nil }
- (id)contentView { this }
- (id)accessoryView { nil }
- (())setAccessoryView:(id)_view {}
- (i32)accessoryType { 0 }
- (())setAccessoryType:(i32)_type {}
- (i32)selectionStyle { 0 }
- (())setSelectionStyle:(i32)_style {}
- (bool)isSelected { false }
- (())setSelected:(bool)_selected {}
- (())setSelected:(bool)_selected animated:(bool)_animated {}
- (bool)isHighlighted { false }
- (())setHighlighted:(bool)_highlighted {}
- (())setHighlighted:(bool)_highlighted animated:(bool)_animated {}
- (())prepareForReuse {}
- (id)backgroundView { nil }
- (())setBackgroundView:(id)_view {}
- (id)selectedBackgroundView { nil }
- (())setSelectedBackgroundView:(id)_view {}
- (())setEditing:(bool)_editing animated:(bool)_animated {}
- (bool)isEditing { false }
- (i32)editingStyle { 0 }
- (bool)showingDeleteConfirmation { false }
- (())setIndentationLevel:(i32)_level {}
- (i32)indentationLevel { 0 }

@end

@implementation UIProgressView: UIView

- (id)initWithFrame:(CGRect)frame {
    msg_super![env; this initWithFrame:frame]
}

- (id)initWithProgressViewStyle:(i32)_style {
    msg_super![env; this init]
}

- (f32)progress { 0.0 }
- (())setProgress:(f32)_progress {}
- (())setProgress:(f32)_progress animated:(bool)_animated {}
- (i32)progressViewStyle { 0 }
- (())setProgressViewStyle:(i32)_style {}
- (id)progressTintColor { nil }
- (())setProgressTintColor:(id)_color {}
- (id)trackTintColor { nil }
- (())setTrackTintColor:(id)_color {}
- (id)progressImage { nil }
- (())setProgressImage:(id)_image {}
- (id)trackImage { nil }
- (())setTrackImage:(id)_image {}

@end

@implementation UITableView: UIScrollView
    
- (id)initWithFrame:(CGRect)frame style:(i32)_style {
    msg_super![env; this initWithFrame:frame]
}

- (id)delegate { nil }
- (())setDelegate:(id)_delegate {}
- (id)dataSource { nil }
- (())setDataSource:(id)_source {}
- (())reloadData {}
- (())reloadSections:(id)_sections withRowAnimation:(i32)_animation {}
- (())reloadRowsAtIndexPaths:(id)_paths withRowAnimation:(i32)_animation {}
- (i32)numberOfSections { 0 }
- (i32)numberOfRowsInSection:(i32)_section { 0 }
- (id)cellForRowAtIndexPath:(id)_path { nil }
- (id)dequeueReusableCellWithIdentifier:(id)_identifier { nil }
- (id)indexPathForCell:(id)_cell { nil }
- (id)indexPathForSelectedRow { nil }
- (id)indexPathsForSelectedRows { nil }
- (())selectRowAtIndexPath:(id)_path animated:(bool)_animated scrollPosition:(i32)_pos {}
- (())deselectRowAtIndexPath:(id)_path animated:(bool)_animated {}
- (())scrollToRowAtIndexPath:(id)_path atScrollPosition:(i32)_pos animated:(bool)_animated {}
- (())setEditing:(bool)_editing animated:(bool)_animated {}
- (bool)isEditing { false }
- (())setAllowsSelection:(bool)_allows {}
- (bool)allowsSelection { true }
- (())setAllowsMultipleSelection:(bool)_allows {}
- (())setRowHeight:(f32)_height {}
- (f32)rowHeight { 44.0 }
- (())setSectionHeaderHeight:(f32)_height {}
- (())setSectionFooterHeight:(f32)_height {}
- (id)tableHeaderView { nil }
- (())setTableHeaderView:(id)_view {}
- (id)tableFooterView { nil }
- (())setTableFooterView:(id)_view {}
- (())setSeparatorStyle:(i32)_style {}
- (())setSeparatorColor:(id)_color {}
- (())registerClass:(id)_class forCellReuseIdentifier:(id)_identifier {}
- (())registerNib:(id)_nib forCellReuseIdentifier:(id)_identifier {}
- (())beginUpdates {}
- (())endUpdates {}
- (())insertRowsAtIndexPaths:(id)_paths withRowAnimation:(i32)_animation {}
- (())deleteRowsAtIndexPaths:(id)_paths withRowAnimation:(i32)_animation {}

@end

@implementation UITableViewController: UIViewController

- (id)initWithStyle:(i32)_style {
    msg_super![env; this init]
}

// Честный iOS-подход: переопределяем создание вьюхи по умолчанию.
// Если NIB'а нет, мы принудительно создаем UITableView, а не UIView.
- (())loadView {
    let frame = <CGRect as Default>::default();
    let table_view: id = msg_class![env; UITableView alloc];
    
    // 0 = UITableViewStylePlain
    let table_view: id = msg![env; table_view initWithFrame:frame style:0];
    
    // Устанавливаем таблицу как главную view этого контроллера
    () = msg![env; this setView:table_view];
    release(env, table_view);
}

// Теперь tableView ссылается на реальный UITableView
- (id)tableView {
    msg![env; this view]
}

- (())setTableView:(id)view {
    () = msg![env; this setView:view];
}

- (bool)clearsSelectionOnViewWillAppear { true }
- (())setClearsSelectionOnViewWillAppear:(bool)_clears {}

@end
    
};
